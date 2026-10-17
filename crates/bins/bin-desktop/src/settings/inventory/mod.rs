//! The **Inventory** Settings page (ADR-0037, and its amendment's one folder per page):
//! `SettingsPage::Inventory`, the Ledger's Properties and, under each open one, its Rooms.
//! [`InventoryPage`] reads its rows from [`LedgerStore`], renders itself (`render.rs`), handles its
//! keys and movement, builds its Dialogs (`dialogs/`) and reaches `Shell` only through
//! [`ViewEvent`]; `Shell`'s Dialog host draws them and confirms them through `apply.rs`. The
//! Settings View forwards to it while the page is open.
//!
//! The Inventory destination (`Noun::Inventory`) is a placeholder that shares nothing with the
//! page: both read the `gpui`-free model through `crate::inventory`, a re-export of
//! `lib_inventory`.
//!
//! Selection is one flat list over the visible Property and Room rows; **+ Add room** is never
//! part of it.

pub mod apply;
pub mod dialogs;
pub mod form;
pub(crate) mod render;

use std::collections::HashSet;

use chrono::NaiveDate;
use gpui::{App, Context, Entity, EventEmitter, Keystroke, Subscription};
use lib_core::{DateStyle, PropertyId, RoomId, UnitKind};
use lib_ledger::{Domain, MoveRoom};
use lib_toast::ToastKind;

use crate::{
    chrome::{dialog_host::OpenDialog, statusline::PageStatus},
    inventory::{self, Inventory},
    ledger_store::{LedgerStore, notify_on},
    navigation::key_router::Movement,
    settings::hints::settings_inventory_hints,
    view::event::ViewEvent,
};
use apply::InventoryOutcome;
use dialogs::InventoryDialog;
use form::{PropertyContext, PropertyForm, RemovePropertyForm, RemoveRoomForm, RoomForm};
pub use render::scope_text;

/// One selectable row: a Property, or a Room of an expanded Property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryRow {
    Property(PropertyId),
    Room(RoomId),
}

/// The rows `j`/`k` walk, in screen order: each Property, then its Rooms when it is expanded.
pub fn visible_rows(inventory: &Inventory, expanded: &HashSet<PropertyId>) -> Vec<InventoryRow> {
    let mut rows = Vec::new();
    for property in &inventory.properties {
        rows.push(InventoryRow::Property(property.id));
        if expanded.contains(&property.id) {
            rows.extend(property.rooms.iter().map(|r| InventoryRow::Room(r.id)));
        }
    }
    rows
}

impl EventEmitter<ViewEvent> for InventoryPage {}

/// Emitted when a click lands on the page, so the Settings View, which owns the page's focus,
/// moves it from the index into the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowClicked;

impl EventEmitter<RowClicked> for InventoryPage {}

/// The Inventory page's own state: the row under the cursor and the Properties shown open.
pub struct InventoryPage {
    ledger: Entity<LedgerStore>,
    /// The date a Property's Renews on is checked against.
    today: NaiveDate,
    /// How a typed Renews on is read, told by `Shell` with the other Display Preferences.
    date_style: Option<DateStyle>,
    /// It may name a row that is gone or hidden, so read it through [`Self::selected_row`].
    selected: Option<InventoryRow>,
    expanded: HashSet<PropertyId>,
    /// Whether the page, not Settings' index, has keyboard focus, told by the Settings View as it
    /// draws the page.
    pub(crate) page_focused: bool,
    /// Repaints the page when the Inventory its rows read changes, or the Units its amounts are
    /// in.
    _inventory_changed: Subscription,
    _units_changed: Subscription,
}

impl InventoryPage {
    pub fn new(ledger: Entity<LedgerStore>, today: NaiveDate, cx: &mut Context<'_, Self>) -> Self {
        let _inventory_changed = notify_on(&ledger, Domain::Inventory, cx);
        let _units_changed = notify_on(&ledger, Domain::Units, cx);
        Self {
            ledger,
            today,
            date_style: None,
            selected: None,
            expanded: HashSet::new(),
            page_focused: false,
            _inventory_changed,
            _units_changed,
        }
    }

    pub(crate) fn inventory<'a>(&self, cx: &'a App) -> &'a Inventory {
        self.ledger.read(cx).ledger().inventory()
    }

    pub fn set_page_focused(&mut self, focused: bool) {
        self.page_focused = focused;
    }

    pub fn set_date_style(&mut self, date_style: Option<DateStyle>) {
        self.date_style = date_style;
    }

    // -----------------------------------------------------------------------------------------
    // The interface the Settings View forwards to while the page is open (ADR-0037)
    // -----------------------------------------------------------------------------------------

    /// `J`/`K` move the selected Room a place within its Property (inert on a Property row), and
    /// `left` closes an open Property or climbs from a Room to it. The router and Settings' focus
    /// keys would otherwise take them, so the page claims them ahead of both; a `left` with
    /// nothing to close or climb is left to Settings, which steps back to the index.
    pub fn key_ahead(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform {
            return false;
        }
        if modifiers.shift {
            let delta = match keystroke.key.as_str() {
                "j" => 1,
                "k" => -1,
                _ => return false,
            };
            // A Room already at the end of its Property stays put.
            if let Some(InventoryRow::Room(room)) = self.selected_row(cx) {
                self.ledger
                    .update(cx, |store, cx| store.apply(cx, MoveRoom { room, delta }))
                    .ok();
            }
            return true;
        }
        if keystroke.key == "left" && self.left_is_local(cx) {
            self.step_out(cx);
            return true;
        }
        false
    }

    /// The page's bare `n`, `r`, `e` and `x`, each through the same handler its button uses, and
    /// `right`, which opens a closed Property or steps into an open one's first Room. Returns
    /// whether the key was one of them.
    pub fn handle_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || modifiers.shift {
            return false;
        }
        match keystroke.key.as_str() {
            "n" => self.open_add_property_dialog(cx),
            "r" => self.add_room_for_selection(cx),
            "e" => {
                if let Some(row) = self.selected_row(cx) {
                    self.open_edit_dialog(row, cx);
                }
            }
            "x" => {
                if let Some(row) = self.selected_row(cx) {
                    self.open_remove_dialog(row, cx);
                }
            }
            "right" => self.step_in(cx),
            _ => return false,
        }
        true
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` walk the visible Property and Room rows as one list.
    pub fn apply_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) {
        let Some(step) = movement.step() else {
            return;
        };
        let rows = self.rows(cx);
        if rows.is_empty() {
            return;
        }
        self.selected = lib_list_cursor::step_id(&rows, self.selected_row(cx), step);
        cx.notify();
    }

    /// The page has nothing of its own to close: `Esc` is left to Settings, which steps back to
    /// the index.
    pub fn escape(&mut self, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// The page has no Search: `/` is inert there.
    pub fn start_search(&mut self, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    pub fn search_key(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// The page's legend for the selected row, and its Property, Room and Item counts.
    pub fn page_status(&self, cx: &App) -> Option<PageStatus> {
        Some(PageStatus {
            hints: settings_inventory_hints(self.selected_row(cx)),
            right: scope_text(self.inventory(cx)),
        })
    }

    /// The heading's meta: how many Properties, Rooms and Items.
    pub fn scope_note(&self, cx: &App) -> String {
        scope_text(self.inventory(cx))
    }

    /// The `?` cheat-sheet's keys for the page, by the selected row.
    pub fn hints(&self, cx: &App) -> Vec<(&'static str, String)> {
        settings_inventory_hints(self.selected_row(cx))
    }

    // -----------------------------------------------------------------------------------------
    // Selection and disclosure
    // -----------------------------------------------------------------------------------------

    fn rows(&self, cx: &App) -> Vec<InventoryRow> {
        visible_rows(self.inventory(cx), &self.expanded)
    }

    /// The selected row: the stored one while it is still on screen, else the first row, so the
    /// cursor is never lost. `None` only with no Properties.
    pub fn selected_row(&self, cx: &App) -> Option<InventoryRow> {
        lib_list_cursor::resolve(self.rows(cx), self.selected)
    }

    fn select(&mut self, row: Option<InventoryRow>, cx: &mut Context<'_, Self>) {
        self.selected = row;
        cx.notify();
    }

    /// Shows Property `id` open. Returns whether it was closed before.
    fn expand(&mut self, id: PropertyId, cx: &mut Context<'_, Self>) -> bool {
        cx.notify();
        self.expanded.insert(id)
    }

    /// Shows Property `id` closed. Returns whether it was open before.
    fn collapse(&mut self, id: PropertyId, cx: &mut Context<'_, Self>) -> bool {
        cx.notify();
        self.expanded.remove(&id)
    }

    /// The Property the selected row belongs to.
    fn selected_property(&self, cx: &App) -> Option<PropertyId> {
        match self.selected_row(cx)? {
            InventoryRow::Property(id) => Some(id),
            InventoryRow::Room(id) => self.inventory(cx).room(id).map(|(property, _)| property.id),
        }
    }

    /// Whether `left` has something to do: close an open Property, or climb from a Room to its
    /// Property.
    fn left_is_local(&self, cx: &App) -> bool {
        match self.selected_row(cx) {
            Some(InventoryRow::Room(_)) => true,
            Some(InventoryRow::Property(id)) => self.expanded.contains(&id),
            None => false,
        }
    }

    /// `right`: open a closed Property, or step from an open one to its first Room.
    fn step_in(&mut self, cx: &mut Context<'_, Self>) {
        let Some(InventoryRow::Property(id)) = self.selected_row(cx) else {
            return;
        };
        if self.expand(id, cx) {
            return;
        }
        let first_room = self
            .inventory(cx)
            .property(id)
            .and_then(|property| property.rooms.first().map(|room| room.id));
        if let Some(room) = first_room {
            self.select(Some(InventoryRow::Room(room)), cx);
        }
    }

    /// `left`: climb from a Room to its Property, or close an open Property.
    fn step_out(&mut self, cx: &mut Context<'_, Self>) {
        match self.selected_row(cx) {
            Some(InventoryRow::Room(id)) => {
                let property = self.inventory(cx).room(id).map(|(property, _)| property.id);
                if let Some(property) = property {
                    self.select(Some(InventoryRow::Property(property)), cx);
                }
            }
            Some(InventoryRow::Property(id)) => {
                self.collapse(id, cx);
            }
            None => {}
        }
    }

    /// `r`: Add room for the selected Property (or the selected Room's), opening it so the new
    /// Room shows. With no Property it is a Toast.
    fn add_room_for_selection(&mut self, cx: &mut Context<'_, Self>) {
        let Some(property) = self.selected_property(cx) else {
            if self.selected_row(cx).is_none() {
                cx.emit(ViewEvent::RaiseToast {
                    kind: ToastKind::Info,
                    text: crate::msg::desktop_inventory_toast_no_property(),
                });
            }
            return;
        };
        self.expand(property, cx);
        self.open_add_room_dialog(property, cx);
    }

    /// Selects and opens what a landed Dialog left the cursor on.
    pub fn land(&mut self, outcome: &InventoryOutcome, cx: &mut Context<'_, Self>) {
        match outcome {
            InventoryOutcome::PropertyAdded { id, .. } => {
                self.expand(*id, cx);
                self.select(Some(InventoryRow::Property(*id)), cx);
            }
            InventoryOutcome::PropertySaved { id, .. } => {
                self.select(Some(InventoryRow::Property(*id)), cx);
            }
            InventoryOutcome::PropertyRemoved { id, selected, .. } => {
                self.collapse(*id, cx);
                self.select(*selected, cx);
            }
            InventoryOutcome::RoomAdded { property, id, .. } => {
                self.expand(*property, cx);
                self.select(Some(InventoryRow::Room(*id)), cx);
            }
            InventoryOutcome::RoomSaved { id, .. } => {
                self.select(Some(InventoryRow::Room(*id)), cx);
            }
            InventoryOutcome::RoomRemoved { selected, .. } => {
                self.select(Some(*selected), cx);
            }
            InventoryOutcome::Dismissed => {}
        }
    }

    // -----------------------------------------------------------------------------------------
    // Clicks
    // -----------------------------------------------------------------------------------------

    /// A click on a row: selects it and moves focus into the page.
    pub(crate) fn click_row(&mut self, row: InventoryRow, cx: &mut Context<'_, Self>) {
        self.select(Some(row), cx);
        cx.emit(RowClicked);
    }

    /// The disclosure control of a Property row: opens or closes it. Closing keeps the selection
    /// on screen by moving a hidden Room's selection up to its Property.
    pub(crate) fn click_toggle(&mut self, id: PropertyId, cx: &mut Context<'_, Self>) {
        cx.emit(RowClicked);
        if self.collapse(id, cx) {
            if let Some(InventoryRow::Room(room)) = self.selected
                && self
                    .inventory(cx)
                    .room(room)
                    .is_some_and(|(property, _)| property.id == id)
            {
                self.select(Some(InventoryRow::Property(id)), cx);
            }
        } else {
            self.expand(id, cx);
        }
    }

    /// **edit** on a row: selects it and asks for its Edit dialog.
    pub(crate) fn click_edit(&mut self, row: InventoryRow, cx: &mut Context<'_, Self>) {
        self.select(Some(row), cx);
        cx.emit(RowClicked);
        self.open_edit_dialog(row, cx);
    }

    /// **remove** on a row: selects it and asks for its Remove dialog.
    pub(crate) fn click_remove(&mut self, row: InventoryRow, cx: &mut Context<'_, Self>) {
        self.select(Some(row), cx);
        cx.emit(RowClicked);
        self.open_remove_dialog(row, cx);
    }

    /// **+ Add property**: the same handler `n` reaches.
    pub(crate) fn click_add_property(&mut self, cx: &mut Context<'_, Self>) {
        cx.emit(RowClicked);
        self.open_add_property_dialog(cx);
    }

    /// **+ Add room** under an open Property, or **+ room** on its row.
    pub(crate) fn click_add_room(&mut self, id: PropertyId, cx: &mut Context<'_, Self>) {
        cx.emit(RowClicked);
        self.open_add_room_dialog(id, cx);
    }

    // -----------------------------------------------------------------------------------------
    // Dialogs
    // -----------------------------------------------------------------------------------------

    /// The fiat Units a Property can be kept in, as option labels, with each one's code.
    fn unit_choices(&self, cx: &App) -> Vec<(String, String)> {
        self.ledger
            .read(cx)
            .ledger()
            .units()
            .iter()
            .filter(|unit| unit.kind == UnitKind::Fiat)
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

    /// What the Property form checks against, copied in when it opens.
    fn property_context(&self, cx: &App) -> PropertyContext {
        PropertyContext {
            inventory: self.inventory(cx).clone(),
            today: self.today,
            date_style: self.date_style,
            unit_choices: self.unit_choices(cx),
        }
    }

    /// `e` or **edit**: a Property opens its dialog; a Room opens its own.
    fn open_edit_dialog(&mut self, row: InventoryRow, cx: &mut Context<'_, Self>) {
        match row {
            InventoryRow::Property(id) => self.open_edit_property_dialog(id, cx),
            InventoryRow::Room(id) => self.open_edit_room_dialog(id, cx),
        }
    }

    /// `x` or **remove**: same split as [`Self::open_edit_dialog`].
    fn open_remove_dialog(&mut self, row: InventoryRow, cx: &mut Context<'_, Self>) {
        match row {
            InventoryRow::Property(id) => self.open_remove_property_dialog(id, cx),
            InventoryRow::Room(id) => self.open_remove_room_dialog(id, cx),
        }
    }

    fn open_add_property_dialog(&mut self, cx: &mut Context<'_, Self>) {
        // The selected Property's Unit, else the first fiat Unit.
        let choices = self.unit_choices(cx);
        let preferred = self
            .selected_property(cx)
            .and_then(|id| self.inventory(cx).property(id))
            .and_then(|property| choices.iter().find(|(_, code)| *code == property.unit))
            .or_else(|| choices.first())
            .map(|(label, _)| label.clone());
        let form = PropertyForm::for_add(preferred, self.property_context(cx));
        open_dialog(InventoryDialog::Add(form), cx);
    }

    fn open_edit_property_dialog(&mut self, id: PropertyId, cx: &mut Context<'_, Self>) {
        let Some(property) = self.inventory(cx).property(id) else {
            return;
        };
        let form = PropertyForm::from_property(property, self.property_context(cx));
        open_dialog(InventoryDialog::Edit(id, form), cx);
    }

    fn open_remove_property_dialog(&mut self, id: PropertyId, cx: &mut Context<'_, Self>) {
        let inventory = self.inventory(cx);
        let Some(property) = inventory.property(id) else {
            return;
        };
        let form = RemovePropertyForm::new(
            property.name.clone(),
            dialogs::needs_typed_name(inventory, id),
        );
        open_dialog(InventoryDialog::Remove(id, form), cx);
    }

    fn open_add_room_dialog(&mut self, property: PropertyId, cx: &mut Context<'_, Self>) {
        let inventory = self.inventory(cx);
        if inventory.property(property).is_none() {
            return;
        }
        let form = RoomForm::for_add(inventory.clone(), property);
        open_dialog(InventoryDialog::AddRoom(property, form), cx);
    }

    fn open_edit_room_dialog(&mut self, id: RoomId, cx: &mut Context<'_, Self>) {
        let inventory = self.inventory(cx);
        let Some((property, room)) = inventory.room(id) else {
            return;
        };
        let form = RoomForm::for_edit(inventory.clone(), property.id, room);
        open_dialog(InventoryDialog::EditRoom(id, form), cx);
    }

    /// An empty Room gets a plain confirm; one with Items a destination picker pre-selecting the
    /// Room above (below when first); the last Room with Items a notice.
    fn open_remove_room_dialog(&mut self, id: RoomId, cx: &mut Context<'_, Self>) {
        let inventory = self.inventory(cx);
        let Some((property, _)) = inventory.room(id) else {
            return;
        };
        let items = inventory.room_items(id);
        let dialog = if items > 0 && property.rooms.len() == 1 {
            InventoryDialog::RoomBlocked(id)
        } else {
            let preselected = (items > 0)
                .then(|| inventory::default_destination(inventory, id))
                .flatten()
                .and_then(|to| inventory.room(to))
                .map(|(_, room)| room.name.clone());
            let destinations = room_destinations(inventory, id);
            InventoryDialog::RemoveRoom(
                id,
                RemoveRoomForm::new(destinations, preselected, items > 0),
            )
        };
        open_dialog(dialog, cx);
    }

    // -----------------------------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------------------------

    /// An Inventory row as `property:<name>` or `room:<name>`.
    fn row_label(&self, row: InventoryRow, cx: &App) -> String {
        let inventory = self.inventory(cx);
        match row {
            InventoryRow::Property(id) => inventory
                .property(id)
                .map(|property| format!("property:{}", property.name))
                .unwrap_or_default(),
            InventoryRow::Room(id) => inventory
                .room(id)
                .map(|(_, room)| format!("room:{}", room.name))
                .unwrap_or_default(),
        }
    }

    /// The visible rows and the selected one, each Property's Rooms, Units and insurers, and each
    /// Room's Item count. The open Dialog is `Shell`'s, which fills it in.
    #[doc(hidden)]
    pub fn snapshot(&self, cx: &App) -> SettingsInventorySnapshot {
        let inventory = self.inventory(cx);
        SettingsInventorySnapshot {
            inventory_rows: self
                .rows(cx)
                .into_iter()
                .map(|row| self.row_label(row, cx))
                .collect(),
            selected_inventory: self.selected_row(cx).map(|row| self.row_label(row, cx)),
            inventory_rooms: inventory
                .properties
                .iter()
                .map(|property| {
                    (
                        property.name.clone(),
                        property
                            .rooms
                            .iter()
                            .map(|room| room.name.clone())
                            .collect(),
                    )
                })
                .collect(),
            inventory_dialog: None,
            inventory_property_ids: inventory.properties.iter().map(|p| p.id).collect(),
            inventory_properties: inventory
                .properties
                .iter()
                .map(|p| {
                    (
                        p.name.clone(),
                        p.unit.clone(),
                        p.cover.as_ref().map(|c| c.insurer.clone()),
                    )
                })
                .collect(),
            inventory_room_items: inventory
                .properties
                .iter()
                .flat_map(|p| p.rooms.iter())
                .map(|room| (room.name.clone(), inventory.room_items(room.id)))
                .collect(),
        }
    }
}

/// The Inventory page's visible rows and the selected one, as `property:<name>` or `room:<name>`,
/// each Property's Rooms in order, and the dialog last asked for.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsInventorySnapshot {
    pub inventory_rows: Vec<String>,
    pub selected_inventory: Option<String>,
    pub inventory_rooms: Vec<(String, Vec<String>)>,
    /// The open dialog's kind (`add`, `edit`, `remove`, `add-room`, ...), which `Shell` fills in,
    /// and each Property as `(name, unit code, insurer)`.
    pub inventory_dialog: Option<String>,
    pub inventory_properties: Vec<(String, String, Option<String>)>,
    /// Each Property's id, in the same order.
    pub inventory_property_ids: Vec<PropertyId>,
    /// Each Room's name with its Item count, in page order.
    pub inventory_room_items: Vec<(String, usize)>,
}

/// The names of the Property's other Rooms, in tab order: where a removed Room's Items can go.
fn room_destinations(inventory: &Inventory, room: RoomId) -> Vec<String> {
    inventory.room(room).map_or_else(Vec::new, |(property, _)| {
        property
            .rooms
            .iter()
            .filter(|r| r.id != room)
            .map(|r| r.name.clone())
            .collect()
    })
}

/// Asks `Shell`'s Dialog host to open an Inventory dialog.
fn open_dialog(dialog: InventoryDialog, cx: &mut Context<'_, InventoryPage>) {
    cx.emit(ViewEvent::OpenDialog(Box::new(OpenDialog::Inventory(
        dialog,
    ))));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed() -> Inventory {
        inventory::default_inventory(NaiveDate::from_ymd_opt(2026, 10, 2).unwrap())
    }

    #[test]
    fn collapsed_properties_hide_their_rooms() {
        let inventory = seed();
        let rows = visible_rows(&inventory, &HashSet::new());
        assert_eq!(rows.len(), 2);
        let first = &inventory.properties[0];
        let rows = visible_rows(&inventory, &HashSet::from([first.id]));
        assert_eq!(rows.len(), 2 + first.rooms.len());
        assert_eq!(rows[1], InventoryRow::Room(first.rooms[0].id));
    }
}
