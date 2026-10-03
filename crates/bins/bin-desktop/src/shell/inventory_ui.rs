//! The Settings › Inventory page's Add, Edit and Remove **Property** dialogs (#494): opening them,
//! their keys and clicks, applying the change to the in-memory Inventory and Documents, and
//! building their elements. The rules live in `inventory` and `inventory_form`; the chrome in
//! `view::settings::inventory_dialogs`. Room dialogs are a later ticket.

use std::rc::Rc;

use gpui::{AnyElement, Context, Keystroke};
use lib_toast::ToastKind;

use super::{InventoryStub, Shell, typed_char};
use crate::{
    documents,
    inventory::{self, PropertyError},
    inventory_form::{self, Problem, PropertyField, PropertyForm},
    nav::InputMode,
    view::settings::{inventory::InventoryRow, inventory_dialogs as view},
};

/// The open Property dialog. `InputMode::Dialog` for exactly as long as it is `Some`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum InventoryDialog {
    Add(PropertyForm),
    Edit(u32, PropertyForm),
    /// The Property and the name typed so far, which only a Property holding something asks for.
    Remove(u32, String),
}

/// What Remove would take with it.
struct Holdings {
    rooms: usize,
    items: usize,
    documents: usize,
}

/// The dialog's status-line legend: Remove has no fields to `tab` through.
pub(super) fn dialog_hints(dialog: &InventoryDialog) -> Vec<(&'static str, String)> {
    let mut hints = vec![
        ("esc", crate::msg::desktop_hint_cancel()),
        ("enter", crate::msg::desktop_hint_confirm()),
    ];
    if !matches!(dialog, InventoryDialog::Remove(..)) {
        hints.push(("tab", crate::msg::desktop_hint_next_field()));
    }
    hints
}

impl Shell {
    /// The fiat Units a Property can be kept in, as option labels, with each one's code.
    fn property_unit_choices(&self) -> Vec<(String, String)> {
        self.settings_units
            .iter()
            .filter(|unit| unit.kind == "currency")
            .map(|unit| {
                (
                    crate::msg::desktop_inventory_unit_option(
                        &unit.code.to_uppercase(),
                        &unit.name,
                    ),
                    unit.code.clone(),
                )
            })
            .collect()
    }

    fn property_unit_labels(&self) -> Vec<String> {
        self.property_unit_choices()
            .into_iter()
            .map(|(label, _)| label)
            .collect()
    }

    /// The Unit code the Add dialog's picker is on, if it has a Unit at all.
    fn property_unit_code(&self, form: &PropertyForm) -> Option<String> {
        let label = form.unit.value()?;
        self.property_unit_choices()
            .into_iter()
            .find(|(option, _)| option == label)
            .map(|(_, code)| code)
    }

    /// The Property the selected row belongs to.
    fn selected_inventory_property(&self) -> Option<u32> {
        match self.settings_inventory_selected_row()? {
            InventoryRow::Property(id) => Some(id),
            InventoryRow::Room(id) => self.inventory.room(id).map(|(property, _)| property.id),
        }
    }

    /// `e` or **edit**: a Property opens its dialog; a Room is still the placeholder.
    pub(super) fn open_edit_inventory_row(&mut self, row: InventoryRow) {
        match row {
            InventoryRow::Property(id) => self.open_edit_property_dialog(id),
            InventoryRow::Room(_) => self.settings_inventory_stub = Some(InventoryStub::Edit(row)),
        }
    }

    /// `x` or **remove**: same split as [`Self::open_edit_inventory_row`].
    pub(super) fn open_remove_inventory_row(&mut self, row: InventoryRow) {
        match row {
            InventoryRow::Property(id) => self.open_remove_property_dialog(id),
            InventoryRow::Room(_) => {
                self.settings_inventory_stub = Some(InventoryStub::Remove(row));
            }
        }
    }

    pub(super) fn open_add_property_dialog(&mut self) {
        // The selected Property's Unit, else the first fiat Unit.
        let choices = self.property_unit_choices();
        let preferred = self
            .selected_inventory_property()
            .and_then(|id| self.inventory.property(id))
            .and_then(|property| choices.iter().find(|(_, code)| *code == property.unit))
            .or_else(|| choices.first())
            .map(|(label, _)| label.clone());
        self.inventory_dialog = Some(InventoryDialog::Add(PropertyForm::for_add(preferred)));
        self.nav.enter_mode(InputMode::Dialog);
    }

    pub(super) fn open_edit_property_dialog(&mut self, id: u32) {
        let Some(property) = self.inventory.property(id) else {
            return;
        };
        let form = PropertyForm::from_property(property);
        self.inventory_dialog = Some(InventoryDialog::Edit(id, form));
        self.nav.enter_mode(InputMode::Dialog);
    }

    pub(super) fn open_remove_property_dialog(&mut self, id: u32) {
        if self.inventory.property(id).is_none() {
            return;
        }
        self.inventory_dialog = Some(InventoryDialog::Remove(id, String::new()));
        self.nav.enter_mode(InputMode::Dialog);
    }

    fn close_inventory_dialog(&mut self) {
        self.inventory_dialog = None;
        self.nav.exit_mode();
    }

    /// The first `Esc` on an open Unit list closes the list only.
    pub(super) fn close_open_property_unit_select(&mut self) -> bool {
        if let Some(InventoryDialog::Add(form)) = self.inventory_dialog.as_mut() {
            let was_open = form.unit.is_open();
            form.unit.cancel();
            return was_open;
        }
        false
    }

    fn holdings(&self, id: u32) -> Holdings {
        let rooms = self.inventory.property(id).map_or(0, |p| p.rooms.len());
        let items: Vec<u32> = self
            .inventory
            .items
            .iter()
            .filter(|item| item.property == id)
            .map(|item| item.id)
            .collect();
        let documents = self
            .documents
            .iter()
            .filter(|document| {
                document.links.iter().any(|link| {
                    matches!(link, documents::DocumentLink::InventoryItem(item) if items.contains(item))
                })
            })
            .count();
        Holdings {
            rooms,
            items: items.len(),
            documents,
        }
    }

    /// The first problem with the open Add or Edit form.
    fn property_problem(&self, form: &PropertyForm, own_id: Option<u32>) -> Option<Problem> {
        let unit = self.property_unit_code(form);
        form.problem(
            &self.inventory,
            own_id,
            unit.as_deref(),
            self.today,
            self.settings_date_style,
        )
    }

    /// Keys while a Property dialog is open. `Esc` never reaches here.
    pub(super) fn handle_inventory_dialog_key(&mut self, keystroke: &Keystroke) -> bool {
        let key = keystroke.key.as_str();
        let shift = keystroke.modifiers.shift;
        let unit_labels = self.property_unit_labels();
        match self.inventory_dialog.as_mut() {
            Some(InventoryDialog::Remove(id, typed)) => {
                let id = *id;
                match key {
                    "enter" => self.confirm_inventory_dialog(),
                    "backspace" => {
                        typed.pop();
                    }
                    "tab" => {}
                    _ => {
                        let Some(ch) = typed_char(keystroke) else {
                            return false;
                        };
                        if self.holdings_need_typed_name(id)
                            && let Some(InventoryDialog::Remove(_, typed)) =
                                self.inventory_dialog.as_mut()
                        {
                            typed.push(ch);
                        }
                    }
                }
                true
            }
            Some(InventoryDialog::Add(form) | InventoryDialog::Edit(_, form)) => {
                let on_unit = form.focused == PropertyField::Unit;
                match key {
                    "tab" => {
                        form.unit.cancel();
                        form.cycle_focus(shift);
                    }
                    "up" | "down" if on_unit => {
                        let delta = if key == "up" { -1 } else { 1 };
                        if form.unit.is_open() {
                            form.unit.move_highlight(&unit_labels, delta);
                        } else {
                            form.unit.step(&unit_labels, delta);
                        }
                    }
                    "space" if on_unit => {
                        if form.unit.is_open() {
                            form.unit.commit(&unit_labels);
                        } else {
                            form.unit.open(&unit_labels);
                        }
                    }
                    "enter" if on_unit && form.unit.is_open() => form.unit.commit(&unit_labels),
                    "enter" => self.confirm_inventory_dialog(),
                    "backspace" => form.backspace(),
                    _ => {
                        let Some(ch) = typed_char(keystroke) else {
                            return false;
                        };
                        form.push_char(ch);
                    }
                }
                true
            }
            None => false,
        }
    }

    fn holdings_need_typed_name(&self, id: u32) -> bool {
        let holdings = self.holdings(id);
        holdings.rooms > 0 || holdings.items > 0
    }

    /// **Add property** / **Save** / **Remove property** and `enter`: a no-op while the dialog is
    /// not valid; otherwise applies the change, selects the Property and closes the dialog.
    pub(super) fn confirm_inventory_dialog(&mut self) {
        let Some(dialog) = self.inventory_dialog.clone() else {
            return;
        };
        match dialog {
            InventoryDialog::Add(form) => {
                if self.property_problem(&form, None).is_some() {
                    return;
                }
                let Some(unit) = self.property_unit_code(&form) else {
                    return;
                };
                let Ok(draft) = form.draft(self.today, self.settings_date_style) else {
                    return;
                };
                let Ok(id) = inventory::add_property(&mut self.inventory, &draft, &unit) else {
                    return;
                };
                self.settings_inventory_expanded.insert(id);
                self.settings_inventory_selected = Some(InventoryRow::Property(id));
                self.raise_toast(
                    ToastKind::Success,
                    crate::msg::desktop_inventory_toast_property_added(form.name.trim()),
                );
            }
            InventoryDialog::Edit(id, form) => {
                if self.property_problem(&form, Some(id)).is_some() {
                    return;
                }
                let Ok(draft) = form.draft(self.today, self.settings_date_style) else {
                    return;
                };
                if inventory::edit_property(&mut self.inventory, id, &draft).is_err() {
                    return;
                }
                self.settings_inventory_selected = Some(InventoryRow::Property(id));
                self.raise_toast(
                    ToastKind::Success,
                    crate::msg::desktop_inventory_toast_property_saved(form.name.trim()),
                );
            }
            InventoryDialog::Remove(id, typed) => {
                let Some(property) = self.inventory.property(id) else {
                    return;
                };
                let name = property.name.clone();
                if self.holdings_need_typed_name(id) && typed != name {
                    return;
                }
                // The cursor lands on the neighbour that takes the removed row's place.
                let position = self
                    .inventory
                    .properties
                    .iter()
                    .position(|p| p.id == id)
                    .unwrap_or(0);
                let removed = match inventory::remove_property(&mut self.inventory, id) {
                    Ok(removed) => removed,
                    Err(PropertyError::Unknown) => return,
                    Err(_) => return,
                };
                documents::drop_inventory_links(&mut self.documents, &removed.items);
                self.settings_inventory_expanded.remove(&id);
                self.settings_inventory_selected = self
                    .inventory
                    .properties
                    .get(position.min(self.inventory.properties.len().saturating_sub(1)))
                    .map(|p| InventoryRow::Property(p.id));
                self.raise_toast(
                    ToastKind::Success,
                    crate::msg::desktop_inventory_toast_property_removed(&name),
                );
            }
        }
        self.close_inventory_dialog();
    }

    fn with_property_form(
        &mut self,
        cx: &mut Context<'_, Self>,
        change: impl FnOnce(&mut PropertyForm),
    ) {
        if let Some(InventoryDialog::Add(form) | InventoryDialog::Edit(_, form)) =
            self.inventory_dialog.as_mut()
        {
            change(form);
        }
        cx.notify();
    }

    fn handle_inventory_dialog_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.close_inventory_dialog();
        cx.notify();
    }

    fn handle_inventory_dialog_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_inventory_dialog();
        cx.notify();
    }

    fn handle_property_unit_field_click(&mut self, cx: &mut Context<'_, Self>) {
        let labels = self.property_unit_labels();
        self.with_property_form(cx, |form| {
            form.focus(PropertyField::Unit);
            if form.unit.is_open() {
                form.unit.cancel();
            } else {
                form.unit.open(&labels);
            }
        });
    }

    fn handle_property_unit_option_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let labels = self.property_unit_labels();
        self.with_property_form(cx, |form| form.unit.choose(&labels, index));
    }

    /// The open dialog as an element, if any.
    pub(super) fn render_inventory_dialog(
        &self,
        entity: &gpui::Entity<Shell>,
        cx: &gpui::App,
    ) -> Option<AnyElement> {
        let plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        match self.inventory_dialog.as_ref()? {
            InventoryDialog::Add(form) => {
                let problem = self.property_problem(form, None);
                Some(self.render_property_form(entity, form, None, problem, plain, cx))
            }
            InventoryDialog::Edit(id, form) => {
                let property = self.inventory.property(*id)?;
                let fixed =
                    crate::msg::desktop_inventory_field_unit_fixed(&property.unit.to_uppercase());
                let problem = self.property_problem(form, Some(*id));
                Some(self.render_property_form(entity, form, Some(fixed), problem, plain, cx))
            }
            InventoryDialog::Remove(id, typed) => {
                let property = self.inventory.property(*id)?;
                let holdings = self.holdings(*id);
                Some(view::render_remove(
                    view::RemoveProps {
                        name: &property.name,
                        rooms: holdings.rooms,
                        items: holdings.items,
                        documents: holdings.documents,
                        typed,
                        handlers: view::RemoveHandlers {
                            on_cancel: plain(Shell::handle_inventory_dialog_cancel),
                            on_confirm: plain(Shell::handle_inventory_dialog_confirm),
                        },
                    },
                    cx,
                ))
            }
        }
    }

    fn render_property_form(
        &self,
        entity: &gpui::Entity<Shell>,
        form: &PropertyForm,
        fixed_unit: Option<String>,
        problem: Option<Problem>,
        plain: impl Fn(fn(&mut Shell, &mut Context<'_, Shell>)) -> crate::dialog::OnClick,
        cx: &gpui::App,
    ) -> AnyElement {
        let on_field_click: view::OnFieldClick = {
            let entity = entity.clone();
            Rc::new(move |field, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.with_property_form(cx, |form| form.focus(field));
                });
            })
        };
        let on_suggestion_click: view::OnSuggestionClick = {
            let entity = entity.clone();
            Rc::new(move |insurer, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.with_property_form(cx, |form| {
                        form.focus(PropertyField::Insurer);
                        form.set_insurer(&insurer);
                    });
                });
            })
        };
        let on_unit_option_click: crate::view::accounts::select_field::OnOptionClick = {
            let entity = entity.clone();
            Rc::new(move |index, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_property_unit_option_click(index, cx);
                });
            })
        };
        let suggestions = inventory_form::suggestions(&self.inventory, &form.insurer);
        view::render_form(
            view::FormProps {
                form,
                fixed_unit,
                unit_options: &self.property_unit_labels(),
                suggestions: &suggestions,
                valid: problem.is_none(),
                problem,
                handlers: view::FormHandlers {
                    on_field_click,
                    on_unit_click: plain(Shell::handle_property_unit_field_click),
                    on_unit_option_click,
                    on_suggestion_click,
                    on_cancel: plain(Shell::handle_inventory_dialog_cancel),
                    on_confirm: plain(Shell::handle_inventory_dialog_confirm),
                },
            },
            cx,
        )
    }
}
