//! The Inventory page's Dialogs: Add, Edit and Remove a **Location** (`location.rs`) and a
//! **Area** (`area.rs`), and the notice the last Area holding Items gets instead of a Remove.
//! [`InventoryDialog`] is the value `Shell`'s Dialog host holds while one is open
//! (`OpenDialog::Inventory`); [`InventoryDialog::render`] draws whichever is open, and every click
//! inside it reaches the form through the handlers `Shell` builds.

pub mod area;
pub mod location;

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_core::{AreaId, ItemId, LocationId};

use super::form::{AreaForm, LocationField, LocationForm, RemoveAreaForm, RemoveLocationForm};
use crate::{
    chrome::dialog_host::{Dialog, DialogKey, DialogOutcome},
    dialog::OnClick,
    documents::{Document, DocumentLink},
    form::{field::TextField, select_field::OnOptionClick},
    inventory::Inventory,
    theme::color,
};
use location::OnFieldClick;

/// The dialog open over the Inventory page: Location and Area dialogs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InventoryDialog {
    Add(LocationForm),
    Edit(LocationId, LocationForm),
    /// The Location and the name typed so far, which only a Location holding something asks for.
    Remove(LocationId, RemoveLocationForm),
    /// Add area to the Location.
    AddArea(LocationId, AreaForm),
    EditArea(AreaId, AreaForm),
    /// The Area, and the chosen destination for its Items (unused while it is empty).
    RemoveArea(AreaId, RemoveAreaForm),
    /// The Area is its Location's last and holds Items.
    AreaBlocked(AreaId),
}

impl InventoryDialog {
    fn inner(&self) -> Option<&dyn Dialog> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => Some(form),
            Self::Remove(_, form) => Some(form),
            Self::AddArea(_, form) | Self::EditArea(_, form) => Some(form),
            Self::RemoveArea(_, form) => Some(form),
            Self::AreaBlocked(_) => None,
        }
    }

    fn inner_mut(&mut self) -> Option<&mut dyn Dialog> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => Some(form),
            Self::Remove(_, form) => Some(form),
            Self::AddArea(_, form) | Self::EditArea(_, form) => Some(form),
            Self::RemoveArea(_, form) => Some(form),
            Self::AreaBlocked(_) => None,
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

    /// The legend: only Add location has several fields (name and Unit) to `tab` through.
    fn hints(&self) -> Option<Vec<(&'static str, String)>> {
        let mut hints = vec![
            ("esc", crate::msg::desktop_hint_cancel()),
            ("enter", crate::msg::desktop_hint_confirm()),
        ];
        if matches!(self, Self::Add(_)) {
            hints.push(("tab", crate::msg::desktop_hint_next_field()));
        }
        Some(hints)
    }
}

/// Every click the open Inventory dialog can make. `Shell` builds it once and each dialog takes
/// the part it draws.
pub struct InventoryDialogHandlers {
    pub on_field_click: OnFieldClick,
    pub on_unit_field_click: OnClick,
    pub on_unit_option_click: OnOptionClick,
    pub on_area_name_click: OnClick,
    pub on_destination_field_click: OnClick,
    pub on_destination_option_click: OnOptionClick,
    pub on_cancel: OnClick,
    pub on_confirm: OnClick,
}

/// What Remove would take with it.
struct Holdings {
    areas: usize,
    items: usize,
    documents: usize,
}

/// The Areas and Items Location `id` holds, and the Documents that Link to one of its Items.
fn holdings(inventory: &Inventory, documents: &[Document], id: LocationId) -> Holdings {
    let areas = inventory.location(id).map_or(0, |p| p.areas.len());
    let items: Vec<ItemId> = inventory
        .items
        .iter()
        .filter(|item| item.location == id)
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
        areas,
        items: items.len(),
        documents,
    }
}

/// Whether removing Location `id` asks for its name typed: only when it holds Areas or Items.
pub(super) fn needs_typed_name(inventory: &Inventory, id: LocationId) -> bool {
    let holdings = holdings(inventory, &[], id);
    holdings.areas > 0 || holdings.items > 0
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
    fn location_form_mut(&mut self) -> Option<&mut LocationForm> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => Some(form),
            _ => None,
        }
    }

    /// A click on a field of the Location form focuses it.
    pub fn click_field(&mut self, field: LocationField) {
        if let Some(form) = self.location_form_mut() {
            form.focus(field);
        }
    }

    pub fn click_unit_field(&mut self) {
        if let Some(form) = self.location_form_mut() {
            form.click_unit();
        }
    }

    pub fn click_unit_option(&mut self, index: usize) {
        if let Some(form) = self.location_form_mut() {
            form.choose_unit(index);
        }
    }

    /// A click on the Remove area dialog's destination field opens or closes its select.
    pub fn click_destination_field(&mut self) {
        if let Self::RemoveArea(_, form) = self {
            form.click_select();
        }
    }

    pub fn click_destination_option(&mut self, index: usize) {
        if let Self::RemoveArea(_, form) = self {
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
            Self::AddArea(..) => "add-area",
            Self::EditArea(..) => "edit-area",
            Self::RemoveArea(..) => "remove-area",
            Self::AreaBlocked(_) => "area-blocked",
        }
    }

    /// Draws the open dialog. Its Location or Area is read from `inventory`; one that has gone
    /// draws nothing rather than panicking mid-render.
    pub fn render(
        &self,
        inventory: &Inventory,
        documents: &[Document],
        handlers: InventoryDialogHandlers,
        cx: &App,
    ) -> Option<AnyElement> {
        let area_form = |adding: bool, location: &str, form: &AreaForm| {
            area::render_area_form(
                area::AreaFormProps {
                    adding,
                    location,
                    form,
                    problem: form.problem(),
                    handlers: area::AreaFormHandlers {
                        on_name_click: handlers.on_area_name_click.clone(),
                        on_cancel: handlers.on_cancel.clone(),
                        on_confirm: handlers.on_confirm.clone(),
                    },
                },
                cx,
            )
        };
        let location_form = |form: &LocationForm, fixed_unit: Option<String>| {
            location::render_form(
                location::FormProps {
                    form,
                    fixed_unit,
                    unit_options: &form.unit_labels(),
                    problem: form.problem(),
                    handlers: location::FormHandlers {
                        on_field_click: handlers.on_field_click.clone(),
                        on_unit_click: handlers.on_unit_field_click.clone(),
                        on_unit_option_click: handlers.on_unit_option_click.clone(),
                        on_cancel: handlers.on_cancel.clone(),
                        on_confirm: handlers.on_confirm.clone(),
                    },
                },
                cx,
            )
        };
        match self {
            Self::AddArea(location, form) => {
                let name = &inventory.location(*location)?.name;
                Some(area_form(true, name, form))
            }
            Self::EditArea(id, form) => {
                let (location, _) = inventory.area(*id)?;
                Some(area_form(false, &location.name, form))
            }
            Self::RemoveArea(id, form) => {
                let (_, area) = inventory.area(*id)?;
                Some(area::render_area_remove(
                    area::AreaRemoveProps {
                        name: &area.name,
                        items: inventory.area_items(*id),
                        destinations: &form.destinations,
                        destination: &form.select,
                        handlers: area::AreaRemoveHandlers {
                            on_field_click: handlers.on_destination_field_click.clone(),
                            on_option_click: handlers.on_destination_option_click.clone(),
                            on_cancel: handlers.on_cancel.clone(),
                            on_confirm: handlers.on_confirm.clone(),
                        },
                    },
                    cx,
                ))
            }
            Self::AreaBlocked(id) => {
                let (location, area) = inventory.area(*id)?;
                Some(area::render_area_blocked(
                    &area.name,
                    &location.name,
                    inventory.area_items(*id),
                    handlers.on_cancel.clone(),
                    cx,
                ))
            }
            Self::Add(form) => Some(location_form(form, None)),
            Self::Edit(id, form) => {
                let location = inventory.location(*id)?;
                let fixed =
                    crate::msg::desktop_inventory_field_unit_fixed(&location.unit.to_uppercase());
                Some(location_form(form, Some(fixed)))
            }
            Self::Remove(id, form) => {
                let location = inventory.location(*id)?;
                let holdings = holdings(inventory, documents, *id);
                Some(location::render_remove(
                    location::RemoveProps {
                        name: &location.name,
                        areas: holdings.areas,
                        items: holdings.items,
                        documents: holdings.documents,
                        typed: form.typed.text(),
                        handlers: location::RemoveHandlers {
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
