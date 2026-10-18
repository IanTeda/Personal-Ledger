//! The **Inventory** Settings page (ADR-0037, and its amendment's one folder per page):
//! `SettingsPage::Inventory`, the Ledger's Locations and, under each open one, its Areas.
//! [`InventoryPage`] reads its rows from [`LedgerStore`], renders itself (`render.rs`), handles its
//! keys and movement, builds its Dialogs (`dialogs/`) and reaches `Shell` only through
//! [`ViewEvent`]. Its Dialogs draw and confirm themselves through the Dialog host, and confirm
//! through `apply.rs`; the page takes the outcome back in [`InventoryPage::land`]. The Settings
//! View forwards to it while the page is open.
//!
//! The Inventory destination (`Noun::Inventory`) is a placeholder that shares nothing with the
//! page: both read the `gpui`-free model through `crate::inventory`, a re-export of
//! `lib_inventory`.
//!
//! Selection is one flat list over the visible Location and Area rows. A row carries no buttons
//! (16q, the Units table principles): a double-click edits it and a right-click opens its
//! [`RowMenu`], whose actions are the same handlers the keys reach.

pub mod apply;
pub mod dialogs;
pub mod form;
pub(crate) mod render;

use std::collections::HashSet;

use gpui::{App, Context, Entity, EventEmitter, Keystroke, Pixels, Point, Subscription};
use lib_core::{AreaId, LocationId, UnitKind};
use lib_ledger::{Domain, MoveArea};
use lib_toast::ToastKind;

use crate::{
    chrome::{dialog_host::open_dialog, statusline::PageStatus},
    inventory::{self, Inventory},
    ledger_store::{LedgerStore, notify_on},
    navigation::key_router::Movement,
    settings::hints::settings_inventory_hints,
    view::event::ViewEvent,
};
use apply::InventoryOutcome;
use dialogs::InventoryDialog;
use form::{AreaForm, LocationContext, LocationForm, RemoveAreaForm, RemoveLocationForm};
pub use render::scope_text;

/// One selectable row: a Location, or an Area of an expanded Location.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryRow {
    Location(LocationId),
    Area(AreaId),
}

/// An entry of a row's context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    AddArea,
    EditLocation,
    RemoveLocation,
    EditArea,
    MoveUp,
    MoveDown,
    RemoveArea,
}

impl MenuAction {
    /// A row's entries in menu order, the destructive one last.
    pub fn for_row(row: InventoryRow) -> &'static [Self] {
        match row {
            InventoryRow::Location(_) => &[Self::AddArea, Self::EditLocation, Self::RemoveLocation],
            InventoryRow::Area(_) => &[
                Self::EditArea,
                Self::MoveUp,
                Self::MoveDown,
                Self::RemoveArea,
            ],
        }
    }

    pub fn label(self) -> String {
        match self {
            Self::AddArea => crate::msg::desktop_inventory_menu_add_area(),
            Self::EditLocation => crate::msg::desktop_inventory_menu_edit_location(),
            Self::RemoveLocation => crate::msg::desktop_inventory_menu_remove_location(),
            Self::EditArea => crate::msg::desktop_inventory_menu_edit_area(),
            Self::MoveUp => crate::msg::desktop_inventory_menu_move_up(),
            Self::MoveDown => crate::msg::desktop_inventory_menu_move_down(),
            Self::RemoveArea => crate::msg::desktop_inventory_menu_remove_area(),
        }
    }

    /// Drawn in red below a rule.
    pub fn is_destructive(self) -> bool {
        matches!(self, Self::RemoveLocation | Self::RemoveArea)
    }

    /// The entry's `debug_selector` suffix and snapshot name.
    pub fn slug(self) -> &'static str {
        match self {
            Self::AddArea => "add-area",
            Self::EditLocation => "edit-location",
            Self::RemoveLocation => "remove-location",
            Self::EditArea => "edit-area",
            Self::MoveUp => "move-up",
            Self::MoveDown => "move-down",
            Self::RemoveArea => "remove-area",
        }
    }
}

/// The context menu open on a row, drawn at the pointer that opened it (window coordinates).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowMenu {
    pub row: InventoryRow,
    pub position: Point<Pixels>,
}

/// The rows `j`/`k` walk, in screen order: each Location, then its Areas when it is expanded.
pub fn visible_rows(inventory: &Inventory, expanded: &HashSet<LocationId>) -> Vec<InventoryRow> {
    let mut rows = Vec::new();
    for location in &inventory.locations {
        rows.push(InventoryRow::Location(location.id));
        if expanded.contains(&location.id) {
            rows.extend(location.areas.iter().map(|r| InventoryRow::Area(r.id)));
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

/// The Inventory page's own state: the row under the cursor, the Locations shown open and the
/// row menu, if one is open.
pub struct InventoryPage {
    ledger: Entity<LedgerStore>,
    /// It may name a row that is gone or hidden, so read it through [`Self::selected_row`].
    selected: Option<InventoryRow>,
    expanded: HashSet<LocationId>,
    menu: Option<RowMenu>,
    /// Whether the page, not Settings' index, has keyboard focus, told by the Settings View as it
    /// draws the page.
    pub(crate) page_focused: bool,
    /// Repaints the page when the Inventory its rows read changes, or the Units its amounts are
    /// in.
    _inventory_changed: Subscription,
    _units_changed: Subscription,
}

impl InventoryPage {
    pub fn new(ledger: Entity<LedgerStore>, cx: &mut Context<'_, Self>) -> Self {
        let _inventory_changed = notify_on(&ledger, Domain::Inventory, cx);
        let _units_changed = notify_on(&ledger, Domain::Units, cx);
        Self {
            ledger,
            selected: None,
            expanded: HashSet::new(),
            menu: None,
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

    // -----------------------------------------------------------------------------------------
    // The interface the Settings View forwards to while the page is open (ADR-0037)
    // -----------------------------------------------------------------------------------------

    /// `J`/`K` move the selected Area a place within its Location (inert on a Location row), `a`
    /// adds an Area, and `left` closes an open Location or climbs from an Area to it. The router and Settings' focus
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
            self.close_menu(cx);
            if let Some(InventoryRow::Area(area)) = self.selected_row(cx) {
                self.move_area(area, delta, cx);
            }
            return true;
        }
        // `a` is the router's Insert key everywhere else, so the page takes it first.
        if keystroke.key == "a" {
            self.close_menu(cx);
            self.add_area_for_selection(cx);
            return true;
        }
        if keystroke.key == "left" && self.left_is_local(cx) {
            self.step_out(cx);
            return true;
        }
        false
    }

    /// The page's bare `n`, `e` and `x`, each through the same handler its menu entry or button
    /// uses, and `right`, which opens a closed Location or steps into an open one's first
    /// Area. Returns whether the key was one of them.
    pub fn handle_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || modifiers.shift {
            return false;
        }
        if matches!(keystroke.key.as_str(), "n" | "e" | "x" | "right") {
            self.close_menu(cx);
        }
        match keystroke.key.as_str() {
            "n" => self.open_add_location_dialog(cx),
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

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` walk the visible Location and Area rows as one list.
    pub fn apply_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) {
        let Some(step) = movement.step() else {
            return;
        };
        let rows = self.rows(cx);
        if rows.is_empty() {
            return;
        }
        self.close_menu(cx);
        self.selected = lib_list_cursor::step_id(&rows, self.selected_row(cx), step);
        cx.notify();
    }

    /// `Esc` closes an open row menu; otherwise it is left to Settings, which steps back to the
    /// index.
    pub fn escape(&mut self, cx: &mut Context<'_, Self>) -> bool {
        self.close_menu(cx)
    }

    /// The page has no Search: `/` is inert there.
    pub fn start_search(&mut self, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    pub fn search_key(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// The page's legend for the selected row, and its Location, Area and Item counts.
    pub fn page_status(&self, cx: &App) -> Option<PageStatus> {
        Some(PageStatus {
            hints: settings_inventory_hints(self.selected_row(cx)),
            right: scope_text(self.inventory(cx)),
        })
    }

    /// The heading's meta: how many Locations, Areas and Items.
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
    /// cursor is never lost. `None` only with no Locations.
    pub fn selected_row(&self, cx: &App) -> Option<InventoryRow> {
        lib_list_cursor::resolve(self.rows(cx), self.selected)
    }

    fn select(&mut self, row: Option<InventoryRow>, cx: &mut Context<'_, Self>) {
        self.selected = row;
        cx.notify();
    }

    /// Shows Location `id` open. Returns whether it was closed before.
    fn expand(&mut self, id: LocationId, cx: &mut Context<'_, Self>) -> bool {
        cx.notify();
        self.expanded.insert(id)
    }

    /// Shows Location `id` closed. Returns whether it was open before.
    fn collapse(&mut self, id: LocationId, cx: &mut Context<'_, Self>) -> bool {
        cx.notify();
        self.expanded.remove(&id)
    }

    /// The Location the selected row belongs to.
    fn selected_location(&self, cx: &App) -> Option<LocationId> {
        match self.selected_row(cx)? {
            InventoryRow::Location(id) => Some(id),
            InventoryRow::Area(id) => self.inventory(cx).area(id).map(|(location, _)| location.id),
        }
    }

    /// Whether `left` has something to do: close an open Location, or climb from an Area to its
    /// Location.
    fn left_is_local(&self, cx: &App) -> bool {
        match self.selected_row(cx) {
            Some(InventoryRow::Area(_)) => true,
            Some(InventoryRow::Location(id)) => self.expanded.contains(&id),
            None => false,
        }
    }

    /// `right`: open a closed Location, or step from an open one to its first Area.
    fn step_in(&mut self, cx: &mut Context<'_, Self>) {
        let Some(InventoryRow::Location(id)) = self.selected_row(cx) else {
            return;
        };
        if self.expand(id, cx) {
            return;
        }
        let first_area = self
            .inventory(cx)
            .location(id)
            .and_then(|location| location.areas.first().map(|area| area.id));
        if let Some(area) = first_area {
            self.select(Some(InventoryRow::Area(area)), cx);
        }
    }

    /// `left`: climb from an Area to its Location, or close an open Location.
    fn step_out(&mut self, cx: &mut Context<'_, Self>) {
        match self.selected_row(cx) {
            Some(InventoryRow::Area(id)) => {
                let location = self.inventory(cx).area(id).map(|(location, _)| location.id);
                if let Some(location) = location {
                    self.select(Some(InventoryRow::Location(location)), cx);
                }
            }
            Some(InventoryRow::Location(id)) => {
                self.collapse(id, cx);
            }
            None => {}
        }
    }

    /// `a`: Add area for the selected Location (or the selected Area's), opening it so the new
    /// Area shows. With no Location it is a Toast.
    fn add_area_for_selection(&mut self, cx: &mut Context<'_, Self>) {
        let Some(location) = self.selected_location(cx) else {
            if self.selected_row(cx).is_none() {
                cx.emit(ViewEvent::RaiseToast {
                    kind: ToastKind::Info,
                    text: crate::msg::desktop_inventory_toast_no_location(),
                });
            }
            return;
        };
        self.expand(location, cx);
        self.open_add_area_dialog(location, cx);
    }

    /// Selects and opens what a landed Dialog left the cursor on.
    pub fn land(&mut self, outcome: InventoryOutcome, cx: &mut Context<'_, Self>) {
        match &outcome {
            InventoryOutcome::LocationAdded { id, .. } => {
                self.expand(*id, cx);
                self.select(Some(InventoryRow::Location(*id)), cx);
            }
            InventoryOutcome::LocationSaved { id, .. } => {
                self.select(Some(InventoryRow::Location(*id)), cx);
            }
            InventoryOutcome::LocationRemoved { id, selected, .. } => {
                self.collapse(*id, cx);
                self.select(*selected, cx);
            }
            InventoryOutcome::AreaAdded { location, id, .. } => {
                self.expand(*location, cx);
                self.select(Some(InventoryRow::Area(*id)), cx);
            }
            InventoryOutcome::AreaSaved { id, .. } => {
                self.select(Some(InventoryRow::Area(*id)), cx);
            }
            InventoryOutcome::AreaRemoved { selected, .. } => {
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

    /// The disclosure control of a Location row: opens or closes it. Closing keeps the selection
    /// on screen by moving a hidden Area's selection up to its Location.
    pub(crate) fn click_toggle(&mut self, id: LocationId, cx: &mut Context<'_, Self>) {
        cx.emit(RowClicked);
        if self.collapse(id, cx) {
            if let Some(InventoryRow::Area(area)) = self.selected
                && self
                    .inventory(cx)
                    .area(area)
                    .is_some_and(|(location, _)| location.id == id)
            {
                self.select(Some(InventoryRow::Location(id)), cx);
            }
        } else {
            self.expand(id, cx);
        }
    }

    /// A double-click on a row: selects it and asks for its Edit dialog, as `e` does.
    pub(crate) fn double_click_row(&mut self, row: InventoryRow, cx: &mut Context<'_, Self>) {
        self.select(Some(row), cx);
        cx.emit(RowClicked);
        self.open_edit_dialog(row, cx);
    }

    /// A right-click on a row: selects it and opens its menu at the pointer.
    pub(crate) fn open_menu(
        &mut self,
        row: InventoryRow,
        position: Point<Pixels>,
        cx: &mut Context<'_, Self>,
    ) {
        self.select(Some(row), cx);
        cx.emit(RowClicked);
        self.menu = Some(RowMenu { row, position });
    }

    /// Closes the row menu. Returns whether one was open.
    pub(crate) fn close_menu(&mut self, cx: &mut Context<'_, Self>) -> bool {
        cx.notify();
        self.menu.take().is_some()
    }

    /// An entry of the open row menu: closes it and runs the same handler as the entry's key.
    pub(crate) fn click_menu_action(&mut self, action: MenuAction, cx: &mut Context<'_, Self>) {
        let Some(RowMenu { row, .. }) = self.menu.take() else {
            return;
        };
        cx.notify();
        match (action, row) {
            (MenuAction::AddArea, InventoryRow::Location(id)) => {
                self.expand(id, cx);
                self.open_add_area_dialog(id, cx);
            }
            (MenuAction::EditLocation | MenuAction::EditArea, row) => {
                self.open_edit_dialog(row, cx);
            }
            (MenuAction::RemoveLocation | MenuAction::RemoveArea, row) => {
                self.open_remove_dialog(row, cx);
            }
            (MenuAction::MoveUp, InventoryRow::Area(area)) => self.move_area(area, -1, cx),
            (MenuAction::MoveDown, InventoryRow::Area(area)) => self.move_area(area, 1, cx),
            // An entry is only drawn on the kind of row it acts on.
            (MenuAction::AddArea, InventoryRow::Area(_))
            | (MenuAction::MoveUp | MenuAction::MoveDown, InventoryRow::Location(_)) => {}
        }
    }

    /// The square **+** beside LOCATIONS: the same handler `n` reaches.
    pub(crate) fn click_add_location(&mut self, cx: &mut Context<'_, Self>) {
        cx.emit(RowClicked);
        self.open_add_location_dialog(cx);
    }

    /// Moves an Area a place within its Location; one already at that end stays put.
    fn move_area(&mut self, area: AreaId, delta: isize, cx: &mut Context<'_, Self>) {
        self.ledger
            .update(cx, |store, cx| store.apply(cx, MoveArea { area, delta }))
            .ok();
    }

    // -----------------------------------------------------------------------------------------
    // Dialogs
    // -----------------------------------------------------------------------------------------

    /// The fiat Units a Location can be kept in, as option labels, with each one's code.
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

    /// What the Location form checks against, copied in when it opens.
    fn location_context(&self, cx: &App) -> LocationContext {
        LocationContext {
            inventory: self.inventory(cx).clone(),
            unit_choices: self.unit_choices(cx),
        }
    }

    /// `e`, a double-click or **Edit …**: a Location opens its dialog; an Area opens its own.
    fn open_edit_dialog(&mut self, row: InventoryRow, cx: &mut Context<'_, Self>) {
        match row {
            InventoryRow::Location(id) => self.open_edit_location_dialog(id, cx),
            InventoryRow::Area(id) => self.open_edit_area_dialog(id, cx),
        }
    }

    /// `x` or **Remove …**: same split as [`Self::open_edit_dialog`].
    fn open_remove_dialog(&mut self, row: InventoryRow, cx: &mut Context<'_, Self>) {
        match row {
            InventoryRow::Location(id) => self.open_remove_location_dialog(id, cx),
            InventoryRow::Area(id) => self.open_remove_area_dialog(id, cx),
        }
    }

    fn open_add_location_dialog(&mut self, cx: &mut Context<'_, Self>) {
        // The selected Location's Unit, else the first fiat Unit.
        let choices = self.unit_choices(cx);
        let preferred = self
            .selected_location(cx)
            .and_then(|id| self.inventory(cx).location(id))
            .and_then(|location| choices.iter().find(|(_, code)| *code == location.unit))
            .or_else(|| choices.first())
            .map(|(label, _)| label.clone());
        let form = LocationForm::for_add(preferred, self.location_context(cx));
        open_dialog(InventoryDialog::Add(form), Self::land, cx);
    }

    fn open_edit_location_dialog(&mut self, id: LocationId, cx: &mut Context<'_, Self>) {
        let Some(location) = self.inventory(cx).location(id) else {
            return;
        };
        let form = LocationForm::from_location(location, self.location_context(cx));
        open_dialog(InventoryDialog::Edit(id, form), Self::land, cx);
    }

    fn open_remove_location_dialog(&mut self, id: LocationId, cx: &mut Context<'_, Self>) {
        let inventory = self.inventory(cx);
        let Some(location) = inventory.location(id) else {
            return;
        };
        let form = RemoveLocationForm::new(
            location.name.clone(),
            dialogs::needs_typed_name(inventory, id),
        );
        open_dialog(InventoryDialog::Remove(id, form), Self::land, cx);
    }

    fn open_add_area_dialog(&mut self, location: LocationId, cx: &mut Context<'_, Self>) {
        let inventory = self.inventory(cx);
        if inventory.location(location).is_none() {
            return;
        }
        let form = AreaForm::for_add(inventory.clone(), location);
        open_dialog(InventoryDialog::AddArea(location, form), Self::land, cx);
    }

    fn open_edit_area_dialog(&mut self, id: AreaId, cx: &mut Context<'_, Self>) {
        let inventory = self.inventory(cx);
        let Some((location, area)) = inventory.area(id) else {
            return;
        };
        let form = AreaForm::for_edit(inventory.clone(), location.id, area);
        open_dialog(InventoryDialog::EditArea(id, form), Self::land, cx);
    }

    /// An empty Area gets a plain confirm; one with Items a destination picker pre-selecting the
    /// Area above (below when first); the last Area with Items a notice.
    fn open_remove_area_dialog(&mut self, id: AreaId, cx: &mut Context<'_, Self>) {
        let inventory = self.inventory(cx);
        let Some((location, _)) = inventory.area(id) else {
            return;
        };
        let items = inventory.area_items(id);
        let dialog = if items > 0 && location.areas.len() == 1 {
            InventoryDialog::AreaBlocked(id)
        } else {
            let preselected = (items > 0)
                .then(|| inventory::default_destination(inventory, id))
                .flatten()
                .and_then(|to| inventory.area(to))
                .map(|(_, area)| area.name.clone());
            let destinations = area_destinations(inventory, id);
            InventoryDialog::RemoveArea(
                id,
                RemoveAreaForm::new(destinations, preselected, items > 0),
            )
        };
        open_dialog(dialog, Self::land, cx);
    }

    // -----------------------------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------------------------

    /// An Inventory row as `location:<name>` or `area:<name>`.
    fn row_label(&self, row: InventoryRow, cx: &App) -> String {
        let inventory = self.inventory(cx);
        match row {
            InventoryRow::Location(id) => inventory
                .location(id)
                .map(|location| format!("location:{}", location.name))
                .unwrap_or_default(),
            InventoryRow::Area(id) => inventory
                .area(id)
                .map(|(_, area)| format!("area:{}", area.name))
                .unwrap_or_default(),
        }
    }

    /// The visible rows and the selected one, each Location's Areas, Units and insurers, and each
    /// Area's Item count.
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
            inventory_areas: inventory
                .locations
                .iter()
                .map(|location| {
                    (
                        location.name.clone(),
                        location
                            .areas
                            .iter()
                            .map(|area| area.name.clone())
                            .collect(),
                    )
                })
                .collect(),
            inventory_menu: self.menu.map(|menu| {
                (
                    self.row_label(menu.row, cx),
                    MenuAction::for_row(menu.row)
                        .iter()
                        .map(|action| action.slug().to_string())
                        .collect(),
                )
            }),
            inventory_location_ids: inventory.locations.iter().map(|p| p.id).collect(),
            inventory_locations: inventory
                .locations
                .iter()
                .map(|p| {
                    (
                        p.name.clone(),
                        p.unit.clone(),
                        p.cover.as_ref().map(|c| c.insurer.clone()),
                    )
                })
                .collect(),
            inventory_area_items: inventory
                .locations
                .iter()
                .flat_map(|p| p.areas.iter())
                .map(|area| (area.name.clone(), inventory.area_items(area.id)))
                .collect(),
        }
    }
}

/// The Inventory page's visible rows and the selected one, as `location:<name>` or `area:<name>`,
/// each Location's Areas in order.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsInventorySnapshot {
    pub inventory_rows: Vec<String>,
    pub selected_inventory: Option<String>,
    pub inventory_areas: Vec<(String, Vec<String>)>,
    /// The open row menu: the row it is on, as above, and its entries' slugs in order.
    pub inventory_menu: Option<(String, Vec<String>)>,
    pub inventory_locations: Vec<(String, String, Option<String>)>,
    /// Each Location's id, in the same order.
    pub inventory_location_ids: Vec<LocationId>,
    /// Each Area's name with its Item count, in page order.
    pub inventory_area_items: Vec<(String, usize)>,
}

/// The names of the Location's other Areas, in tab order: where a removed Area's Items can go.
fn area_destinations(inventory: &Inventory, area: AreaId) -> Vec<String> {
    inventory.area(area).map_or_else(Vec::new, |(location, _)| {
        location
            .areas
            .iter()
            .filter(|r| r.id != area)
            .map(|r| r.name.clone())
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed() -> Inventory {
        inventory::default_inventory(chrono::NaiveDate::from_ymd_opt(2026, 10, 2).unwrap())
    }

    #[test]
    fn collapsed_locations_hide_their_areas() {
        let inventory = seed();
        let rows = visible_rows(&inventory, &HashSet::new());
        assert_eq!(rows.len(), 3);
        let first = &inventory.locations[0];
        let rows = visible_rows(&inventory, &HashSet::from([first.id]));
        assert_eq!(rows.len(), 3 + first.areas.len());
        assert_eq!(rows[1], InventoryRow::Area(first.areas[0].id));
    }
}
