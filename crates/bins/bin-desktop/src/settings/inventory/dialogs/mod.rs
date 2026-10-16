//! The Inventory page's Dialogs: Add, Edit and Remove a **Property** (`property.rs`) and a
//! **Room** (`room.rs`), and the notice the last Room holding Items gets instead of a Remove.
//! [`InventoryDialog`] is the value `Shell`'s Dialog host holds while one is open
//! (`OpenDialog::Inventory`); [`InventoryDialog::render`] draws whichever is open, and every click
//! inside it reaches the form through the handlers `Shell` builds.

pub mod property;
pub mod room;

use gpui::{AnyElement, App, div, prelude::*, px};

use super::form::{
    PropertyField, PropertyForm, RemovePropertyForm, RemoveRoomForm, RoomForm, suggestions,
};
use crate::{
    chrome::dialog_host::{Dialog, DialogKey, DialogOutcome},
    dialog::OnClick,
    documents::{Document, DocumentLink},
    form::{field::TextField, select_field::OnOptionClick},
    inventory::Inventory,
    theme::color,
};
use property::{OnFieldClick, OnSuggestionClick};

/// The dialog open over the Inventory page: Property and Room dialogs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventoryDialog {
    Add(PropertyForm),
    Edit(u32, PropertyForm),
    /// The Property and the name typed so far, which only a Property holding something asks for.
    Remove(u32, RemovePropertyForm),
    /// Add room to the Property.
    AddRoom(u32, RoomForm),
    EditRoom(u32, RoomForm),
    /// The Room, and the chosen destination for its Items (unused while it is empty).
    RemoveRoom(u32, RemoveRoomForm),
    /// The Room is its Property's last and holds Items.
    RoomBlocked(u32),
}

impl InventoryDialog {
    fn inner(&self) -> Option<&dyn Dialog> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => Some(form),
            Self::Remove(_, form) => Some(form),
            Self::AddRoom(_, form) | Self::EditRoom(_, form) => Some(form),
            Self::RemoveRoom(_, form) => Some(form),
            Self::RoomBlocked(_) => None,
        }
    }

    fn inner_mut(&mut self) -> Option<&mut dyn Dialog> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => Some(form),
            Self::Remove(_, form) => Some(form),
            Self::AddRoom(_, form) | Self::EditRoom(_, form) => Some(form),
            Self::RemoveRoom(_, form) => Some(form),
            Self::RoomBlocked(_) => None,
        }
    }
}

impl Dialog for InventoryDialog {
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        self.inner_mut()?.handle_own_key(key)
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        self.inner_mut()?.focused_text()
    }

    fn cycle_field(&mut self) {
        if let Some(inner) = self.inner_mut() {
            inner.cycle_field();
        }
    }

    fn is_valid(&self) -> bool {
        self.inner().is_none_or(Dialog::is_valid)
    }

    fn close_open_select(&mut self) -> bool {
        self.inner_mut().is_some_and(Dialog::close_open_select)
    }

    /// The legend: only the Property form has several fields to `tab` through.
    fn hints(&self) -> Option<Vec<(&'static str, String)>> {
        let mut hints = vec![
            ("esc", crate::msg::desktop_hint_cancel()),
            ("enter", crate::msg::desktop_hint_confirm()),
        ];
        if matches!(self, Self::Add(_) | Self::Edit(..)) {
            hints.push(("tab", crate::msg::desktop_hint_next_field()));
        }
        Some(hints)
    }
}

/// Every click the open Inventory dialog can make. `Shell` builds it once and each dialog takes
/// the part it draws.
pub struct InventoryDialogHandlers {
    pub on_field_click: OnFieldClick,
    pub on_suggestion_click: OnSuggestionClick,
    pub on_unit_field_click: OnClick,
    pub on_unit_option_click: OnOptionClick,
    pub on_room_name_click: OnClick,
    pub on_destination_field_click: OnClick,
    pub on_destination_option_click: OnOptionClick,
    pub on_cancel: OnClick,
    pub on_confirm: OnClick,
}

/// What Remove would take with it.
struct Holdings {
    rooms: usize,
    items: usize,
    documents: usize,
}

/// The Rooms and Items Property `id` holds, and the Documents that Link to one of its Items.
fn holdings(inventory: &Inventory, documents: &[Document], id: u32) -> Holdings {
    let rooms = inventory.property(id).map_or(0, |p| p.rooms.len());
    let items: Vec<u32> = inventory
        .items
        .iter()
        .filter(|item| item.property == id)
        .map(|item| item.id)
        .collect();
    let documents = documents
        .iter()
        .filter(|document| {
            document.links.iter().any(
                |link| matches!(link, DocumentLink::InventoryItem(item) if items.contains(item)),
            )
        })
        .count();
    Holdings {
        rooms,
        items: items.len(),
        documents,
    }
}

/// Whether removing Property `id` asks for its name typed: only when it holds Rooms or Items.
pub(super) fn needs_typed_name(inventory: &Inventory, id: u32) -> bool {
    let holdings = holdings(inventory, &[], id);
    holdings.rooms > 0 || holdings.items > 0
}

fn count(len: usize) -> i64 {
    i64::try_from(len).unwrap_or(i64::MAX)
}

fn note(text: String, error: bool, cx: &App) -> impl IntoElement {
    div()
        .mt(px(4.0))
        .text_size(px(11.5))
        .text_color(if error {
            color::accent_text(cx)
        } else {
            color::muted(cx)
        })
        .child(text)
}

impl InventoryDialog {
    fn property_form_mut(&mut self) -> Option<&mut PropertyForm> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => Some(form),
            _ => None,
        }
    }

    /// A click on a field of the Property form focuses it.
    pub fn click_field(&mut self, field: PropertyField) {
        if let Some(form) = self.property_form_mut() {
            form.focus(field);
        }
    }

    /// A click on an insurer suggestion fills the Insurer.
    pub fn click_suggestion(&mut self, insurer: &str) {
        if let Some(form) = self.property_form_mut() {
            form.focus(PropertyField::Insurer);
            form.set_insurer(insurer);
        }
    }

    pub fn click_unit_field(&mut self) {
        if let Some(form) = self.property_form_mut() {
            form.click_unit();
        }
    }

    pub fn click_unit_option(&mut self, index: usize) {
        if let Some(form) = self.property_form_mut() {
            form.choose_unit(index);
        }
    }

    /// A click on the Remove room dialog's destination field opens or closes its select.
    pub fn click_destination_field(&mut self) {
        if let Self::RemoveRoom(_, form) = self {
            form.click_select();
        }
    }

    pub fn click_destination_option(&mut self, index: usize) {
        if let Self::RemoveRoom(_, form) = self {
            form.choose(index);
        }
    }

    /// The kind of dialog, for the Settings snapshot.
    #[doc(hidden)]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Add(_) => "add",
            Self::Edit(..) => "edit",
            Self::Remove(..) => "remove",
            Self::AddRoom(..) => "add-room",
            Self::EditRoom(..) => "edit-room",
            Self::RemoveRoom(..) => "remove-room",
            Self::RoomBlocked(_) => "room-blocked",
        }
    }

    /// Draws the open dialog. Its Property or Room is read from `inventory`; one that has gone
    /// draws nothing rather than panicking mid-render.
    pub fn render(
        &self,
        inventory: &Inventory,
        documents: &[Document],
        handlers: InventoryDialogHandlers,
        cx: &App,
    ) -> Option<AnyElement> {
        let room_form = |adding: bool, property: &str, form: &RoomForm| {
            room::render_room_form(
                room::RoomFormProps {
                    adding,
                    property,
                    form,
                    problem: form.problem(),
                    handlers: room::RoomFormHandlers {
                        on_name_click: handlers.on_room_name_click.clone(),
                        on_cancel: handlers.on_cancel.clone(),
                        on_confirm: handlers.on_confirm.clone(),
                    },
                },
                cx,
            )
        };
        let property_form = |form: &PropertyForm, fixed_unit: Option<String>| {
            let problem = form.problem();
            property::render_form(
                property::FormProps {
                    form,
                    fixed_unit,
                    unit_options: &form.unit_labels(),
                    suggestions: &suggestions(inventory, form.insurer.text()),
                    valid: problem.is_none(),
                    problem,
                    handlers: property::FormHandlers {
                        on_field_click: handlers.on_field_click.clone(),
                        on_unit_click: handlers.on_unit_field_click.clone(),
                        on_unit_option_click: handlers.on_unit_option_click.clone(),
                        on_suggestion_click: handlers.on_suggestion_click.clone(),
                        on_cancel: handlers.on_cancel.clone(),
                        on_confirm: handlers.on_confirm.clone(),
                    },
                },
                cx,
            )
        };
        match self {
            Self::AddRoom(property, form) => {
                let name = &inventory.property(*property)?.name;
                Some(room_form(true, name, form))
            }
            Self::EditRoom(id, form) => {
                let (property, _) = inventory.room(*id)?;
                Some(room_form(false, &property.name, form))
            }
            Self::RemoveRoom(id, form) => {
                let (_, room) = inventory.room(*id)?;
                Some(room::render_room_remove(
                    room::RoomRemoveProps {
                        name: &room.name,
                        items: inventory.room_items(*id),
                        destinations: &form.destinations,
                        destination: &form.select,
                        handlers: room::RoomRemoveHandlers {
                            on_field_click: handlers.on_destination_field_click.clone(),
                            on_option_click: handlers.on_destination_option_click.clone(),
                            on_cancel: handlers.on_cancel.clone(),
                            on_confirm: handlers.on_confirm.clone(),
                        },
                    },
                    cx,
                ))
            }
            Self::RoomBlocked(id) => {
                let (property, room) = inventory.room(*id)?;
                Some(room::render_room_blocked(
                    &room.name,
                    &property.name,
                    inventory.room_items(*id),
                    handlers.on_cancel.clone(),
                    cx,
                ))
            }
            Self::Add(form) => Some(property_form(form, None)),
            Self::Edit(id, form) => {
                let property = inventory.property(*id)?;
                let fixed =
                    crate::msg::desktop_inventory_field_unit_fixed(&property.unit.to_uppercase());
                Some(property_form(form, Some(fixed)))
            }
            Self::Remove(id, form) => {
                let property = inventory.property(*id)?;
                let holdings = holdings(inventory, documents, *id);
                Some(property::render_remove(
                    property::RemoveProps {
                        name: &property.name,
                        rooms: holdings.rooms,
                        items: holdings.items,
                        documents: holdings.documents,
                        typed: form.typed.text(),
                        handlers: property::RemoveHandlers {
                            on_cancel: handlers.on_cancel.clone(),
                            on_confirm: handlers.on_confirm.clone(),
                        },
                    },
                    cx,
                ))
            }
        }
    }
}
