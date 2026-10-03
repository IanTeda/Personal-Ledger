//! The Settings › Inventory page's Add, Edit and Remove **Property** (#494) and **Room** (#495)
//! dialogs: opening them,
//! their keys and clicks, applying the change to the in-memory Inventory and Documents, and
//! building their elements. The rules live in `inventory` and `inventory_form`; the chrome in
//! `view::settings::inventory_dialogs`.

use std::rc::Rc;

use gpui::{AnyElement, Context, Keystroke};
use lib_toast::ToastKind;

use super::{Shell, typed_char};
use crate::{
    documents,
    inventory::{self, PropertyError},
    inventory_form::{self, Problem, PropertyField, PropertyForm, RoomForm},
    nav::InputMode,
    select::SelectState,
    view::settings::{inventory::InventoryRow, inventory_dialogs as view},
};

/// The open Property dialog. `InputMode::Dialog` for exactly as long as it is `Some`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum InventoryDialog {
    Add(PropertyForm),
    Edit(u32, PropertyForm),
    /// The Property and the name typed so far, which only a Property holding something asks for.
    Remove(u32, String),
    /// Add room to the Property.
    AddRoom(u32, RoomForm),
    EditRoom(u32, RoomForm),
    /// The Room, and the chosen destination for its Items (unused while it is empty).
    RemoveRoom(u32, SelectState),
    /// The Room is its Property's last and holds Items.
    RoomBlocked(u32),
}

/// What Remove would take with it.
struct Holdings {
    rooms: usize,
    items: usize,
    documents: usize,
}

/// The dialog's status-line legend: only the Property form has several fields to `tab` through.
pub(super) fn dialog_hints(dialog: &InventoryDialog) -> Vec<(&'static str, String)> {
    let mut hints = vec![
        ("esc", crate::msg::desktop_hint_cancel()),
        ("enter", crate::msg::desktop_hint_confirm()),
    ];
    if matches!(dialog, InventoryDialog::Add(_) | InventoryDialog::Edit(..)) {
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

    /// `e` or **edit**: a Property opens its dialog; a Room opens its own.
    pub(super) fn open_edit_inventory_row(&mut self, row: InventoryRow) {
        match row {
            InventoryRow::Property(id) => self.open_edit_property_dialog(id),
            InventoryRow::Room(id) => self.open_edit_room_dialog(id),
        }
    }

    /// `x` or **remove**: same split as [`Self::open_edit_inventory_row`].
    pub(super) fn open_remove_inventory_row(&mut self, row: InventoryRow) {
        match row {
            InventoryRow::Property(id) => self.open_remove_property_dialog(id),
            InventoryRow::Room(id) => self.open_remove_room_dialog(id),
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

    pub(super) fn open_add_room_dialog(&mut self, property: u32) {
        if self.inventory.property(property).is_none() {
            return;
        }
        self.inventory_dialog = Some(InventoryDialog::AddRoom(property, RoomForm::default()));
        self.nav.enter_mode(InputMode::Dialog);
    }

    fn open_edit_room_dialog(&mut self, id: u32) {
        let Some((_, room)) = self.inventory.room(id) else {
            return;
        };
        let form = RoomForm::for_edit(room);
        self.inventory_dialog = Some(InventoryDialog::EditRoom(id, form));
        self.nav.enter_mode(InputMode::Dialog);
    }

    /// An empty Room gets a plain confirm; one with Items a destination picker pre-selecting the
    /// Room above (below when first); the last Room with Items a notice.
    fn open_remove_room_dialog(&mut self, id: u32) {
        let Some((property, _)) = self.inventory.room(id) else {
            return;
        };
        let dialog = if self.inventory.room_items(id) == 0 {
            InventoryDialog::RemoveRoom(id, SelectState::new(None))
        } else if property.rooms.len() == 1 {
            InventoryDialog::RoomBlocked(id)
        } else {
            let preselected = inventory::default_destination(&self.inventory, id)
                .and_then(|to| self.inventory.room(to))
                .map(|(_, room)| room.name.clone());
            InventoryDialog::RemoveRoom(id, SelectState::new(preselected))
        };
        self.inventory_dialog = Some(dialog);
        self.nav.enter_mode(InputMode::Dialog);
    }

    fn close_inventory_dialog(&mut self) {
        self.inventory_dialog = None;
        self.nav.exit_mode();
    }

    /// The first `Esc` on an open Unit list closes the list only.
    pub(super) fn close_open_property_unit_select(&mut self) -> bool {
        match self.inventory_dialog.as_mut() {
            Some(InventoryDialog::Add(form)) => {
                let was_open = form.unit.is_open();
                form.unit.cancel();
                was_open
            }
            Some(InventoryDialog::RemoveRoom(_, select)) => {
                let was_open = select.is_open();
                select.cancel();
                was_open
            }
            _ => false,
        }
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
            Some(InventoryDialog::AddRoom(_, form) | InventoryDialog::EditRoom(_, form)) => {
                match key {
                    "enter" => self.confirm_inventory_dialog(),
                    "backspace" => form.backspace(),
                    "tab" => {}
                    _ => {
                        let Some(ch) = typed_char(keystroke) else {
                            return false;
                        };
                        form.push_char(ch);
                    }
                }
                true
            }
            Some(InventoryDialog::RemoveRoom(id, select)) => {
                let list = room_destinations(&self.inventory, *id);
                match key {
                    "up" | "down" => {
                        let delta = if key == "up" { -1 } else { 1 };
                        if select.is_open() {
                            select.move_highlight(&list, delta);
                        } else {
                            select.step(&list, delta);
                        }
                    }
                    "space" if !select.is_open() => select.open(&list),
                    "space" => select.commit(&list),
                    "enter" if select.is_open() => select.commit(&list),
                    "enter" => self.confirm_inventory_dialog(),
                    _ => return false,
                }
                true
            }
            Some(InventoryDialog::RoomBlocked(_)) => {
                if key == "enter" {
                    self.close_inventory_dialog();
                    return true;
                }
                false
            }
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
            InventoryDialog::AddRoom(property, form) => {
                if form.problem(&self.inventory, property, None).is_some() {
                    return;
                }
                let Ok(id) = inventory::add_room(&mut self.inventory, property, &form.name) else {
                    return;
                };
                self.settings_inventory_expanded.insert(property);
                self.settings_inventory_selected = Some(InventoryRow::Room(id));
                self.raise_toast(
                    ToastKind::Success,
                    crate::msg::desktop_inventory_toast_room_added(form.name.trim()),
                );
            }
            InventoryDialog::EditRoom(id, form) => {
                let Some((property, _)) = self.inventory.room(id) else {
                    return;
                };
                if form
                    .problem(&self.inventory, property.id, Some(id))
                    .is_some()
                {
                    return;
                }
                if inventory::edit_room(&mut self.inventory, id, &form.name).is_err() {
                    return;
                }
                self.settings_inventory_selected = Some(InventoryRow::Room(id));
                self.raise_toast(
                    ToastKind::Success,
                    crate::msg::desktop_inventory_toast_room_saved(form.name.trim()),
                );
            }
            InventoryDialog::RemoveRoom(id, select) => {
                let Some((property, room)) = self.inventory.room(id) else {
                    return;
                };
                let (property_id, name) = (property.id, room.name.clone());
                let position = property.rooms.iter().position(|r| r.id == id).unwrap_or(0);
                let items = self.inventory.room_items(id);
                let destination = select.value().and_then(|chosen| {
                    property
                        .rooms
                        .iter()
                        .find(|r| r.id != id && r.name == chosen)
                });
                if items > 0 && destination.is_none() {
                    return;
                }
                let destination_name = destination.map(|r| r.name.clone());
                let destination_id = destination.map(|r| r.id);
                if inventory::remove_room(&mut self.inventory, id, destination_id).is_err() {
                    return;
                }
                // The cursor lands on the Room that takes the removed row's place, else the Property.
                let rooms = self
                    .inventory
                    .property(property_id)
                    .map(|p| p.rooms.as_slice())
                    .unwrap_or_default();
                self.settings_inventory_selected = Some(
                    rooms
                        .get(position.min(rooms.len().saturating_sub(1)))
                        .map_or(InventoryRow::Property(property_id), |r| {
                            InventoryRow::Room(r.id)
                        }),
                );
                let toast = match destination_name {
                    Some(to) if items > 0 => {
                        crate::msg::desktop_inventory_toast_room_removed_moved(
                            &name,
                            &crate::msg::desktop_inventory_count_items(
                                i64::try_from(items).unwrap_or(i64::MAX),
                            ),
                            &to,
                        )
                    }
                    _ => crate::msg::desktop_inventory_toast_room_removed(&name),
                };
                self.raise_toast(ToastKind::Success, toast);
            }
            InventoryDialog::RoomBlocked(_) => {}
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

    fn handle_room_name_click(&mut self, cx: &mut Context<'_, Self>) {
        cx.notify();
    }

    fn handle_room_destination_field_click(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(InventoryDialog::RemoveRoom(id, select)) = self.inventory_dialog.as_mut() {
            let list = room_destinations(&self.inventory, *id);
            if select.is_open() {
                select.cancel();
            } else {
                select.open(&list);
            }
        }
        cx.notify();
    }

    fn handle_room_destination_option_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        if let Some(InventoryDialog::RemoveRoom(id, select)) = self.inventory_dialog.as_mut() {
            let list = room_destinations(&self.inventory, *id);
            select.choose(&list, index);
        }
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
            InventoryDialog::AddRoom(property, form) => {
                let name = self.inventory.property(*property)?.name.clone();
                Some(self.render_room_form(
                    true,
                    &name,
                    form,
                    form.problem(&self.inventory, *property, None),
                    plain,
                    cx,
                ))
            }
            InventoryDialog::EditRoom(id, form) => {
                let (property, _) = self.inventory.room(*id)?;
                let problem = form.problem(&self.inventory, property.id, Some(*id));
                Some(self.render_room_form(false, &property.name.clone(), form, problem, plain, cx))
            }
            InventoryDialog::RemoveRoom(id, select) => {
                let (_, room) = self.inventory.room(*id)?;
                let destinations = room_destinations(&self.inventory, *id);
                let on_option_click: crate::view::accounts::select_field::OnOptionClick = {
                    let entity = entity.clone();
                    Rc::new(move |index, _window, cx| {
                        entity.update(cx, |shell, cx| {
                            shell.handle_room_destination_option_click(index, cx);
                        });
                    })
                };
                Some(view::render_room_remove(
                    view::RoomRemoveProps {
                        name: &room.name,
                        items: self.inventory.room_items(*id),
                        destinations: &destinations,
                        destination: select,
                        handlers: view::RoomRemoveHandlers {
                            on_field_click: plain(Shell::handle_room_destination_field_click),
                            on_option_click,
                            on_cancel: plain(Shell::handle_inventory_dialog_cancel),
                            on_confirm: plain(Shell::handle_inventory_dialog_confirm),
                        },
                    },
                    cx,
                ))
            }
            InventoryDialog::RoomBlocked(id) => {
                let (property, room) = self.inventory.room(*id)?;
                Some(view::render_room_blocked(
                    &room.name,
                    &property.name,
                    self.inventory.room_items(*id),
                    plain(Shell::handle_inventory_dialog_cancel),
                    cx,
                ))
            }
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

    fn render_room_form(
        &self,
        adding: bool,
        property: &str,
        form: &RoomForm,
        problem: Option<inventory::NameError>,
        plain: impl Fn(fn(&mut Shell, &mut Context<'_, Shell>)) -> crate::dialog::OnClick,
        cx: &gpui::App,
    ) -> AnyElement {
        view::render_room_form(
            view::RoomFormProps {
                adding,
                property,
                form,
                problem,
                handlers: view::RoomFormHandlers {
                    on_name_click: plain(Shell::handle_room_name_click),
                    on_cancel: plain(Shell::handle_inventory_dialog_cancel),
                    on_confirm: plain(Shell::handle_inventory_dialog_confirm),
                },
            },
            cx,
        )
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

/// The names of the Property's other Rooms, in tab order: where a removed Room's Items can go.
fn room_destinations(inventory: &inventory::Inventory, room: u32) -> Vec<String> {
    inventory.room(room).map_or_else(Vec::new, |(property, _)| {
        property
            .rooms
            .iter()
            .filter(|r| r.id != room)
            .map(|r| r.name.clone())
            .collect()
    })
}
