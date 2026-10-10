//! The Settings › Inventory page's Add, Edit and Remove **Property** (#494) and **Room** (#495)
//! dialogs: opening them,
//! their keys and clicks, applying the change to the in-memory Inventory and Documents, and
//! building their elements. The rules live in `inventory` and `inventory::form`; the chrome in
//! `view::inventory::dialogs`.

use std::rc::Rc;

use gpui::{AnyElement, App, Context};
use lib_toast::ToastKind;

use super::Shell;
use crate::{
    chrome::dialog_host::OpenDialog,
    documents,
    inventory::form::{
        InventoryDialog, Problem, PropertyContext, PropertyField, PropertyForm, RemovePropertyForm,
        RemoveRoomForm, RoomForm,
    },
    inventory::{self, PropertyError},
    view::{inventory::dialogs as view, settings::inventory::InventoryRow},
};

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
    fn property_unit_choices(&self, cx: &App) -> Vec<(String, String)> {
        self.units_store
            .read(cx)
            .units()
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

    /// The Inventory, read through its store Entity (ADR-0032).
    pub(super) fn inventory<'a>(&self, cx: &'a App) -> &'a inventory::Inventory {
        self.inventory.read(cx).inventory()
    }

    /// What the Property form checks against, copied in when it opens.
    fn property_context(&self, cx: &App) -> PropertyContext {
        PropertyContext {
            inventory: self.inventory(cx).clone(),
            today: self.today,
            date_style: self.settings_date_style,
            unit_choices: self.property_unit_choices(cx),
        }
    }

    /// The Property the selected row belongs to.
    fn selected_inventory_property(&self, cx: &App) -> Option<u32> {
        match self.settings_inventory_selected_row(cx)? {
            InventoryRow::Property(id) => Some(id),
            InventoryRow::Room(id) => self.inventory(cx).room(id).map(|(property, _)| property.id),
        }
    }

    /// `e` or **edit**: a Property opens its dialog; a Room opens its own.
    pub(super) fn open_edit_inventory_row(&mut self, row: InventoryRow, cx: &App) {
        match row {
            InventoryRow::Property(id) => self.open_edit_property_dialog(id, cx),
            InventoryRow::Room(id) => self.open_edit_room_dialog(id, cx),
        }
    }

    /// `x` or **remove**: same split as [`Self::open_edit_inventory_row`].
    pub(super) fn open_remove_inventory_row(&mut self, row: InventoryRow, cx: &App) {
        match row {
            InventoryRow::Property(id) => self.open_remove_property_dialog(id, cx),
            InventoryRow::Room(id) => self.open_remove_room_dialog(id, cx),
        }
    }

    pub(super) fn open_add_property_dialog(&mut self, cx: &App) {
        // The selected Property's Unit, else the first fiat Unit.
        let choices = self.property_unit_choices(cx);
        let preferred = self
            .selected_inventory_property(cx)
            .and_then(|id| self.inventory(cx).property(id))
            .and_then(|property| choices.iter().find(|(_, code)| *code == property.unit))
            .or_else(|| choices.first())
            .map(|(label, _)| label.clone());
        let form = PropertyForm::for_add(preferred, self.property_context(cx));
        self.open_dialog(OpenDialog::Inventory(InventoryDialog::Add(form)));
    }

    pub(super) fn open_edit_property_dialog(&mut self, id: u32, cx: &App) {
        let Some(property) = self.inventory(cx).property(id) else {
            return;
        };
        let form = PropertyForm::from_property(property, self.property_context(cx));
        self.open_dialog(OpenDialog::Inventory(InventoryDialog::Edit(id, form)));
    }

    pub(super) fn open_remove_property_dialog(&mut self, id: u32, cx: &App) {
        let Some(property) = self.inventory(cx).property(id) else {
            return;
        };
        let form =
            RemovePropertyForm::new(property.name.clone(), self.holdings_need_typed_name(id, cx));
        self.open_dialog(OpenDialog::Inventory(InventoryDialog::Remove(id, form)));
    }

    pub(super) fn open_add_room_dialog(&mut self, property: u32, cx: &App) {
        if self.inventory(cx).property(property).is_none() {
            return;
        }
        let form = RoomForm::for_add(self.inventory(cx).clone(), property);
        self.open_dialog(OpenDialog::Inventory(InventoryDialog::AddRoom(
            property, form,
        )));
    }

    fn open_edit_room_dialog(&mut self, id: u32, cx: &App) {
        let Some((property, room)) = self.inventory(cx).room(id) else {
            return;
        };
        let form = RoomForm::for_edit(self.inventory(cx).clone(), property.id, room);
        self.open_dialog(OpenDialog::Inventory(InventoryDialog::EditRoom(id, form)));
    }

    /// An empty Room gets a plain confirm; one with Items a destination picker pre-selecting the
    /// Room above (below when first); the last Room with Items a notice.
    fn open_remove_room_dialog(&mut self, id: u32, cx: &App) {
        let Some((property, _)) = self.inventory(cx).room(id) else {
            return;
        };
        let items = self.inventory(cx).room_items(id);
        let dialog = if items > 0 && property.rooms.len() == 1 {
            InventoryDialog::RoomBlocked(id)
        } else {
            let preselected = (items > 0)
                .then(|| inventory::default_destination(self.inventory(cx), id))
                .flatten()
                .and_then(|to| self.inventory(cx).room(to))
                .map(|(_, room)| room.name.clone());
            let destinations = room_destinations(self.inventory(cx), id);
            InventoryDialog::RemoveRoom(
                id,
                RemoveRoomForm::new(destinations, preselected, items > 0),
            )
        };
        self.open_dialog(OpenDialog::Inventory(dialog));
    }

    fn holdings(&self, id: u32, cx: &App) -> Holdings {
        let rooms = self.inventory(cx).property(id).map_or(0, |p| p.rooms.len());
        let items: Vec<u32> = self
            .inventory(cx)
            .items
            .iter()
            .filter(|item| item.property == id)
            .map(|item| item.id)
            .collect();
        let documents = self
            .documents(cx)
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

    fn holdings_need_typed_name(&self, id: u32, cx: &App) -> bool {
        let holdings = self.holdings(id, cx);
        holdings.rooms > 0 || holdings.items > 0
    }

    /// Applies a confirmed Inventory dialog (reached through [`Self::confirm_open_dialog`] from
    /// **Add property** / **Save** / **Remove property** and `enter`): applies the change and
    /// selects the Property or Room. The Room-blocked notice only closes.
    pub(super) fn apply_inventory_dialog(&mut self, dialog: InventoryDialog, cx: &mut App) {
        match dialog {
            InventoryDialog::AddRoom(property, form) => {
                let name = form.name.text().trim().to_string();
                let Ok(id) = self
                    .inventory
                    .update(cx, |store, cx| store.add_room(cx, property, &name))
                else {
                    return;
                };
                self.settings_view
                    .update(cx, |v, cx| v.expand(cx, property));
                self.settings_view.update(cx, |v, cx| {
                    v.set_inventory_selected(cx, Some(InventoryRow::Room(id)))
                });
                self.raise_toast(
                    ToastKind::Success,
                    crate::msg::desktop_inventory_toast_room_added(&name),
                );
            }
            InventoryDialog::EditRoom(id, form) => {
                let name = form.name.text().trim().to_string();
                if self
                    .inventory
                    .update(cx, |store, cx| store.edit_room(cx, id, &name))
                    .is_err()
                {
                    return;
                }
                self.settings_view.update(cx, |v, cx| {
                    v.set_inventory_selected(cx, Some(InventoryRow::Room(id)))
                });
                self.raise_toast(
                    ToastKind::Success,
                    crate::msg::desktop_inventory_toast_room_saved(&name),
                );
            }
            InventoryDialog::RemoveRoom(id, form) => {
                let Some((property, room)) = self.inventory(cx).room(id) else {
                    return;
                };
                let (property_id, name) = (property.id, room.name.clone());
                let position = property.rooms.iter().position(|r| r.id == id).unwrap_or(0);
                let items = self.inventory(cx).room_items(id);
                let destination = form.select.value().and_then(|chosen| {
                    property
                        .rooms
                        .iter()
                        .find(|r| r.id != id && r.name == chosen)
                });
                let destination_name = destination.map(|r| r.name.clone());
                let destination_id = destination.map(|r| r.id);
                if self
                    .inventory
                    .update(cx, |store, cx| store.remove_room(cx, id, destination_id))
                    .is_err()
                {
                    return;
                }
                // The cursor lands on the Room that takes the removed row's place, else the Property.
                let rooms = self
                    .inventory(cx)
                    .property(property_id)
                    .map(|p| p.rooms.as_slice())
                    .unwrap_or_default();
                let selected = rooms
                    .get(position.min(rooms.len().saturating_sub(1)))
                    .map_or(InventoryRow::Property(property_id), |r| {
                        InventoryRow::Room(r.id)
                    });
                self.settings_view
                    .update(cx, |v, cx| v.set_inventory_selected(cx, Some(selected)));
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
                let Some(unit) = form.unit_code() else {
                    return;
                };
                let Ok(draft) = form.draft() else {
                    return;
                };
                let Ok(id) = self
                    .inventory
                    .update(cx, |store, cx| store.add_property(cx, &draft, unit))
                else {
                    return;
                };
                self.settings_view.update(cx, |v, cx| v.expand(cx, id));
                self.settings_view.update(cx, |v, cx| {
                    v.set_inventory_selected(cx, Some(InventoryRow::Property(id)))
                });
                self.raise_toast(
                    ToastKind::Success,
                    crate::msg::desktop_inventory_toast_property_added(form.name.text().trim()),
                );
            }
            InventoryDialog::Edit(id, form) => {
                let Ok(draft) = form.draft() else {
                    return;
                };
                if self
                    .inventory
                    .update(cx, |store, cx| store.edit_property(cx, id, &draft))
                    .is_err()
                {
                    return;
                }
                self.settings_view.update(cx, |v, cx| {
                    v.set_inventory_selected(cx, Some(InventoryRow::Property(id)))
                });
                self.raise_toast(
                    ToastKind::Success,
                    crate::msg::desktop_inventory_toast_property_saved(form.name.text().trim()),
                );
            }
            InventoryDialog::Remove(id, _) => {
                let Some(property) = self.inventory(cx).property(id) else {
                    return;
                };
                let name = property.name.clone();
                // The cursor lands on the neighbour that takes the removed row's place.
                let position = self
                    .inventory(cx)
                    .properties
                    .iter()
                    .position(|p| p.id == id)
                    .unwrap_or(0);
                let removed = match self
                    .inventory
                    .update(cx, |store, cx| store.remove_property(cx, id))
                {
                    Ok(removed) => removed,
                    Err(PropertyError::Unknown) => return,
                    Err(_) => return,
                };
                self.mutate_documents(cx, |data| {
                    documents::drop_inventory_links(&mut data.documents, &removed.items)
                });
                self.settings_view.update(cx, |v, cx| v.collapse(cx, id));
                let selected = self
                    .inventory(cx)
                    .properties
                    .get(position.min(self.inventory(cx).properties.len().saturating_sub(1)))
                    .map(|p| InventoryRow::Property(p.id));
                self.settings_view
                    .update(cx, |v, cx| v.set_inventory_selected(cx, selected));
                self.raise_toast(
                    ToastKind::Success,
                    crate::msg::desktop_inventory_toast_property_removed(&name),
                );
            }
        }
    }

    fn with_property_form(
        &mut self,
        cx: &mut Context<'_, Self>,
        change: impl FnOnce(&mut PropertyForm),
    ) {
        if let Some(InventoryDialog::Add(form) | InventoryDialog::Edit(_, form)) =
            self.inventory_dialog_mut()
        {
            change(form);
        }
        cx.notify();
    }

    fn handle_inventory_dialog_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.close_dialog();
        cx.notify();
    }

    fn handle_inventory_dialog_confirm(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_open_dialog(cx);
        cx.notify();
    }

    fn handle_room_name_click(&mut self, cx: &mut Context<'_, Self>) {
        cx.notify();
    }

    fn handle_room_destination_field_click(&mut self, cx: &mut Context<'_, Self>) {
        if let Some(InventoryDialog::RemoveRoom(_, form)) = self.inventory_dialog_mut() {
            form.click_select();
        }
        cx.notify();
    }

    fn handle_room_destination_option_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        if let Some(InventoryDialog::RemoveRoom(_, form)) = self.inventory_dialog_mut() {
            form.choose(index);
        }
        cx.notify();
    }

    fn handle_property_unit_field_click(&mut self, cx: &mut Context<'_, Self>) {
        self.with_property_form(cx, PropertyForm::click_unit);
    }

    fn handle_property_unit_option_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.with_property_form(cx, |form| form.choose_unit(index));
    }

    /// The open dialog as an element, if any.
    pub(super) fn render_inventory_dialog(
        &self,
        entity: &gpui::Entity<Shell>,
        cx: &App,
    ) -> Option<AnyElement> {
        let plain = |handler: fn(&mut Shell, &mut Context<'_, Shell>)| {
            let entity = entity.clone();
            let on_click: crate::dialog::OnClick = Rc::new(move |_window, cx| {
                entity.update(cx, handler);
            });
            on_click
        };
        match self.inventory_dialog()? {
            InventoryDialog::AddRoom(property, form) => {
                let name = self.inventory(cx).property(*property)?.name.clone();
                Some(self.render_room_form(true, &name, form, form.problem(), plain, cx))
            }
            InventoryDialog::EditRoom(id, form) => {
                let (property, _) = self.inventory(cx).room(*id)?;
                let problem = form.problem();
                Some(self.render_room_form(false, &property.name.clone(), form, problem, plain, cx))
            }
            InventoryDialog::RemoveRoom(id, form) => {
                let (_, room) = self.inventory(cx).room(*id)?;
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
                        items: self.inventory(cx).room_items(*id),
                        destinations: &form.destinations,
                        destination: &form.select,
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
                let (property, room) = self.inventory(cx).room(*id)?;
                Some(view::render_room_blocked(
                    &room.name,
                    &property.name,
                    self.inventory(cx).room_items(*id),
                    plain(Shell::handle_inventory_dialog_cancel),
                    cx,
                ))
            }
            InventoryDialog::Add(form) => {
                let problem = form.problem();
                Some(self.render_property_form(entity, form, None, problem, plain, cx))
            }
            InventoryDialog::Edit(id, form) => {
                let property = self.inventory(cx).property(*id)?;
                let fixed =
                    crate::msg::desktop_inventory_field_unit_fixed(&property.unit.to_uppercase());
                let problem = form.problem();
                Some(self.render_property_form(entity, form, Some(fixed), problem, plain, cx))
            }
            InventoryDialog::Remove(id, form) => {
                let property = self.inventory(cx).property(*id)?;
                let holdings = self.holdings(*id, cx);
                Some(view::render_remove(
                    view::RemoveProps {
                        name: &property.name,
                        rooms: holdings.rooms,
                        items: holdings.items,
                        documents: holdings.documents,
                        typed: form.typed.text(),
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
        cx: &App,
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
        cx: &App,
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
        let suggestions = inventory::form::suggestions(self.inventory(cx), form.insurer.text());
        view::render_form(
            view::FormProps {
                form,
                fixed_unit,
                unit_options: &form.unit_labels(),
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
