//! The Inventory Settings page's View: the `lib-inventory` page Presenter in a gpui Entity. It
//! looks up each key the Shell passes on, as chord text, in the Inventory keymap, turns clicks into
//! events, draws the view model (`render.rs`). Each key and click is one input (`crate::input::send`), which
//! commits what it wrote. The Settings View hosts it as any other page.
//!
//! The row menu's position is the View's: the Presenter holds which row the menu is on and its
//! entries, and the pointer that asked for it is gpui's.

mod render;

use gpui::{
    AnyElement, App, Context, Entity, EventEmitter, IntoElement as _, Keystroke, Pixels, Point,
    Subscription,
};
use lib_database::Domain;
use lib_database::StoredInventoryReads;
use lib_inventory::presenter::{
    InventoryPageEvent, InventoryPagePresenter, InventoryPageViewModel, MenuAction,
};
use lib_inventory::{InventoryOutcome, InventoryRow, InventoryUnits as _, SharedInventory};
use lib_viper::keymap::Keymap;

use lib_shell::Hint;

use crate::inventory::ports;
use crate::inventory::router::PageRouter;
use crate::inventory::{InventorySetup, SettingsInventorySnapshot};
use crate::settings::snapshot::PageSnapshot;
use crate::{
    announcer::{Announcer, notify_on},
    navigation::movement::Movement,
    settings::{
        SettingsPage,
        pages::{Page, PageRowClicked},
    },
    shell::view::page_clicks::PresenterPage,
    view::event::ViewEvent,
};

impl EventEmitter<ViewEvent> for InventoryPage {}
impl EventEmitter<PageRowClicked> for InventoryPage {}

pub struct InventoryPage {
    presenter: InventoryPagePresenter,
    inventory: SharedInventory,
    /// The Units and Documents the Dialogs quote, over the stores (#1014).
    reads: StoredInventoryReads,
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
        announcer: Entity<Announcer>,
        cx: &mut Context<'_, Self>,
    ) -> Self {
        let inventory = ports::shared(&setup.store);
        Self {
            presenter: InventoryPagePresenter::new(inventory.clone(), setup.keymap.clone()),
            inventory,
            reads: setup.reads,
            _inventory_changed: notify_on(&announcer, Domain::Inventory, cx),
            _units_changed: notify_on(&announcer, Domain::Units, cx),
            keymap: setup.keymap,
            menu_at: Point::default(),
            page_focused: false,
        }
    }

    /// The one dispatch a keymap event takes into the page, from a key ([`Self::module_key`]) or a palette Command ([`Self::module_command`]): sent as one input, unless the page doesn't take it now.
    fn take(&mut self, event: InventoryPageEvent, cx: &mut Context<'_, Self>) -> bool {
        if !self.presenter.takes(&event) {
            return false;
        }
        self.send(event, cx);
        true
    }

    /// A key or click on the page: `event` as one input, committed once it is handled.
    fn send(&mut self, event: InventoryPageEvent, cx: &mut Context<'_, Self>) {
        crate::input::join_or_send(cx, |cx| self.handle(event, cx));
    }

    /// Hands `event` to the Presenter with the Desktop's Router. Inside an input already, as
    /// [`Self::send`] or a Dialog's confirm.
    fn handle(&mut self, event: InventoryPageEvent, cx: &mut Context<'_, Self>) {
        let mut router = PageRouter {
            inventory: self.inventory.clone(),
            reads: self.reads.clone(),
            cx,
        };
        self.presenter.handle(event, &mut router);
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

/// The page's clicks on its rows, its row menu and its buttons.
impl PresenterPage<InventoryPageEvent> for InventoryPage {
    fn send(&mut self, event: InventoryPageEvent, cx: &mut Context<'_, Self>) {
        InventoryPage::send(self, event, cx);
    }

    /// A click on a row or its controls hands the page focus.
    fn row_clicked(&mut self, cx: &mut Context<'_, Self>) {
        cx.emit(PageRowClicked);
    }

    /// The open row menu is drawn where the pointer opened it.
    fn menu_at(&mut self, at: Point<Pixels>, _cx: &mut Context<'_, Self>) {
        self.menu_at = at;
    }
}

impl PageSnapshot for SettingsInventorySnapshot {
    type Page = InventoryPage;

    /// The visible rows and the selected one, each Location's Areas, Units and insurers, and each
    /// Area's Item count.
    fn read(page: &InventoryPage, _cx: &App) -> Self {
        let inventory = page.inventory.borrow().inventory();
        let view = page.view_model();
        let units = &page.reads;
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
                        units.unit_code(p.unit).unwrap_or_default(),
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
        self.take(event, cx)
    }

    /// A palette Command, by its keymap `command` name: the event its key sends, if the page
    /// takes it now, as [`Self::module_key`] asks.
    fn module_command(&mut self, command: &str, cx: &mut Context<'_, Self>) -> bool {
        let Some(event) = self.keymap.event_for(command) else {
            return false;
        };
        self.take(event, cx)
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
        self.view_model().hints
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
