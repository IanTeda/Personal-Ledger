//! The Inventory Settings page's View: the `lib-inventory` page Presenter in a gpui Entity. It
//! looks up each key the Shell passes on, as chord text, in the Inventory keymap, turns clicks into
//! events, draws the view model (`render.rs`), and keeps the Ledger read copy in line after every
//! event ([`ports::refresh`]). Each key and click is one input (`crate::input::send`), which
//! commits what it wrote. The Settings View hosts it as any other page.
//!
//! The row menu's position is the View's: the Presenter holds which row the menu is on and its
//! entries, and the pointer that asked for it is gpui's.

mod render;

use gpui::{
    AnyElement, App, Context, Entity, EventEmitter, IntoElement as _, Keystroke, Pixels, Point,
    Subscription,
};
use lib_inventory::presenter::{
    InventoryPageEvent, InventoryPagePresenter, InventoryPageViewModel, MenuAction,
};
use lib_inventory::{InventoryOutcome, InventoryRow, SharedInventory};
use lib_ledger::Domain;
use lib_viper::keymap::Keymap;

use crate::inventory::ports;
use crate::inventory::router::PageRouter;
use crate::inventory::{InventorySetup, SettingsInventorySnapshot};
use crate::settings::snapshot::PageSnapshot;
use crate::{
    ledger_store::{LedgerStore, notify_on},
    navigation::movement::Movement,
    settings::{SettingsPage, page::PageRowClicked, pages::Page},
    shell::view::statusline::Hint,
    view::event::ViewEvent,
};

impl EventEmitter<ViewEvent> for InventoryPage {}
impl EventEmitter<PageRowClicked> for InventoryPage {}

pub struct InventoryPage {
    presenter: InventoryPagePresenter,
    inventory: SharedInventory,
    ledger: Entity<LedgerStore>,
    keymap: Keymap<InventoryPageEvent>,
    /// Where the open row menu was asked for, in window coordinates.
    menu_at: Point<Pixels>,
    /// Whether the page, not Settings' index, has keyboard focus, told by the Settings View as it
    /// draws the page.
    page_focused: bool,
    /// Repaints the page when the Inventory its rows read changes, or the Units its amounts are
    /// in.
    _inventory_changed: Subscription,
    _units_changed: Subscription,
}

impl InventoryPage {
    pub fn new(
        setup: InventorySetup,
        ledger: Entity<LedgerStore>,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let inventory = ports::shared(&setup.store);
        Self {
            presenter: InventoryPagePresenter::new(inventory.clone(), setup.keymap.clone()),
            inventory,
            _inventory_changed: notify_on(&ledger, Domain::Inventory, cx),
            _units_changed: notify_on(&ledger, Domain::Units, cx),
            ledger,
            keymap: setup.keymap,
            menu_at: Point::default(),
            page_focused: false,
        }
    }

    /// A key or click on the page: `event` as one input, committed once it is handled.
    pub(in crate::inventory) fn send(
        &mut self,
        event: InventoryPageEvent,
        cx: &mut Context<'_, Self>,
    ) {
        crate::input::send(cx, |cx| self.handle(event, cx));
    }

    /// Hands `event` to the Presenter with the Desktop's Router, then brings the Ledger read copy
    /// in line with whatever it wrote. Inside an input already, as [`Self::send`] or a Dialog's
    /// confirm.
    fn handle(&mut self, event: InventoryPageEvent, cx: &mut Context<'_, Self>) {
        ports::refresh(&self.ledger, cx);
        let mut router = PageRouter {
            inventory: self.inventory.clone(),
            ledger: self.ledger.clone(),
            cx,
        };
        self.presenter.handle(event, &mut router);
        ports::refresh(&self.ledger, cx);
        cx.notify();
    }

    /// A confirmed Dialog's outcome, handed back by the Dialog host inside the confirm's input.
    pub(in crate::inventory) fn dialog_landed(
        &mut self,
        outcome: InventoryOutcome,
        cx: &mut Context<'_, Self>,
    ) {
        self.handle(InventoryPageEvent::DialogClosed(Some(outcome)), cx);
    }

    pub fn view_model(&self) -> InventoryPageViewModel {
        self.presenter.view_model()
    }

    /// A click on a row or its controls: `event`, and focus on the page.
    fn click(&mut self, event: InventoryPageEvent, cx: &mut Context<'_, Self>) {
        cx.emit(PageRowClicked);
        self.send(event, cx);
    }

    /// A right-click on a row: selects it and opens its menu at the pointer.
    fn open_menu(
        &mut self,
        row: InventoryRow,
        position: Point<Pixels>,
        cx: &mut Context<'_, Self>,
    ) {
        self.menu_at = position;
        self.click(InventoryPageEvent::OpenMenu(row), cx);
    }

    /// An `InventoryRow` as `location:<name>` or `area:<name>`.
    fn row_label(&self, row: InventoryRow) -> String {
        let inventory = self.inventory.borrow().inventory();
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
}

impl PageSnapshot for SettingsInventorySnapshot {
    type Page = InventoryPage;

    /// The visible rows and the selected one, each Location's Areas, Units and insurers, and each
    /// Area's Item count.
    fn read(page: &InventoryPage, cx: &App) -> Self {
        let inventory = page.inventory.borrow().inventory();
        let view = page.view_model();
        let ledger = page.ledger.read(cx).ledger();
        SettingsInventorySnapshot {
            inventory_rows: view
                .rows
                .iter()
                .map(|line| page.row_label(line.row))
                .collect(),
            selected_inventory: view.selected.map(|row| page.row_label(row)),
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
            inventory_menu: view.menu.map(|menu| {
                (
                    page.row_label(menu.row),
                    menu.entries
                        .iter()
                        .map(|entry| slug(entry.action).to_string())
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
                        ledger.unit_code(p.unit).to_string(),
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

/// The entry's `debug_selector` suffix and snapshot name.
fn slug(action: MenuAction) -> &'static str {
    match action {
        MenuAction::AddArea => "add-area",
        MenuAction::EditLocation => "edit-location",
        MenuAction::RemoveLocation => "remove-location",
        MenuAction::EditArea => "edit-area",
        MenuAction::MoveUp => "move-up",
        MenuAction::MoveDown => "move-down",
        MenuAction::RemoveArea => "remove-area",
    }
}

impl Page for InventoryPage {
    const PAGE: SettingsPage = SettingsPage::Inventory;

    /// Every chord the Inventory keymap binds, the arrows and `g g` among them, except `left` with
    /// nothing to close or climb to, which is the Settings index's. The Shell has already taken
    /// its own keys.
    fn module_key(&mut self, chord: &str, cx: &mut Context<'_, Self>) -> bool {
        let Some(event) = self.keymap.event(chord) else {
            return false;
        };
        if !self.presenter.takes(&event) {
            return false;
        }
        self.send(event, cx);
        true
    }

    /// A palette Command, by its keymap `command` name: the event its key sends, if the page
    /// takes it now, as [`Self::module_key`] asks.
    fn module_command(&mut self, command: &str, cx: &mut Context<'_, Self>) -> bool {
        let Some(event) = self.keymap.event_for(command) else {
            return false;
        };
        if !self.presenter.takes(&event) {
            return false;
        }
        self.send(event, cx);
        true
    }

    /// A movement the keymap doesn't bind (`enter`) does nothing on the page, and Settings leaves
    /// the page where it is: there is no hand-off from a row.
    fn apply_movement(&mut self, _movement: Movement, _cx: &mut Context<'_, Self>) -> bool {
        true
    }

    /// `Esc` closes an open row menu; otherwise it is left to Settings, which steps back to the
    /// index.
    fn escape(&mut self, cx: &mut Context<'_, Self>) -> bool {
        if !self.presenter.menu_open() {
            return false;
        }
        self.send(InventoryPageEvent::CloseMenu, cx);
        true
    }

    /// Nothing the keymap doesn't bind is the page's.
    fn handle_key(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// The Presenter's legend, rebound keys included, which the `?` cheat sheet reads.
    fn hints(&self, _cx: &App) -> Vec<Hint> {
        legend(self.view_model().hints)
    }

    fn status_note(&self, _cx: &App) -> String {
        self.view_model().scope
    }

    fn set_page_focused(&mut self, focused: bool, _cx: &mut Context<'_, Self>) {
        self.page_focused = focused;
    }

    fn heading_meta(&self, _cx: &App) -> AnyElement {
        self.view_model().scope.into_any_element()
    }
}

/// The Presenter's hints as the status line and the `?` cheat sheet take them.
fn legend(hints: Vec<lib_inventory::presenter::Hint>) -> Vec<Hint> {
    hints
        .into_iter()
        .map(|hint| (hint.chord, hint.action))
        .collect()
}
