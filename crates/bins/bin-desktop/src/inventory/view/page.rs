//! The Inventory Settings page's View: the `lib-inventory` page Presenter's [`PageView`], which the
//! generic `ModulePage` hosts (ADR-0056). It names the page's Router, the domains it repaints on,
//! and how it draws the view model (`render.rs`); the keymap, the take, send and handle path and
//! the Dialogs' landing are the generic page's. Where the pointer opened a row's menu is the
//! View's widget state.

mod render;

use gpui::{AnyElement, App, Context, IntoElement as _, Pixels, Point, Window};
use lib_database::Domain;
use lib_inventory::presenter::{
    InventoryPageEvent, InventoryPagePresenter, InventoryPageViewModel, MenuAction,
};
use lib_inventory::{Inventory, InventoryRow, InventoryUnits as _};

use crate::inventory::router::PageRouter;
use crate::inventory::{InventoryPage, InventoryRoutes, SettingsInventorySnapshot};
use crate::settings::SettingsPage;
use crate::settings::snapshot::PageSnapshot;
use crate::shell::view::module_page::{PageDraw, PageView, SettingsPageView};
use crate::shell::view::statusline::PageStatus;

impl PageView for InventoryPagePresenter {
    type Setup = InventoryRoutes;
    /// Where the open row menu is drawn: the pointer that opened it, in window coordinates.
    type Widgets = Point<Pixels>;
    type Before = ();

    /// An Inventory write, or the Units its amounts are in.
    const REPAINTS: &'static [Domain] = &[Domain::Inventory, Domain::Units];

    fn with_router<R>(
        presenter: &mut Self,
        setup: &InventoryRoutes,
        cx: &mut Context<'_, InventoryPage>,
        handle: impl FnOnce(&mut Self, &mut Self::Router<'_>) -> R,
    ) -> R {
        handle(presenter, &mut PageRouter { routes: setup, cx })
    }

    fn render(
        view: &InventoryPageViewModel,
        page: PageDraw<'_, Self>,
        _window: &mut Window,
        cx: &mut Context<'_, InventoryPage>,
    ) -> AnyElement {
        render::page(view, *page.widgets, page.clicks, page.focused, cx)
    }

    /// The Presenter's legend, rebound keys included, and its scope.
    fn status(view: &InventoryPageViewModel) -> PageStatus {
        PageStatus {
            hints: view.hints.clone(),
            right: view.scope.clone(),
        }
    }

    fn heading_meta(view: &InventoryPageViewModel, _cx: &App) -> Option<AnyElement> {
        Some(view.scope.clone().into_any_element())
    }

    /// `Esc` closes an open row menu; otherwise it is left to Settings.
    fn escape(view: &InventoryPageViewModel) -> Option<InventoryPageEvent> {
        view.menu.is_some().then_some(InventoryPageEvent::CloseMenu)
    }

    /// The open row menu is drawn where the pointer opened it.
    fn menu_at(widgets: &mut Point<Pixels>, at: Point<Pixels>) {
        *widgets = at;
    }
}

impl SettingsPageView for InventoryPagePresenter {
    const PAGE: SettingsPage = SettingsPage::Inventory;
}

/// An `InventoryRow` as `location:<name>` or `area:<name>`.
fn row_label(inventory: &Inventory, row: InventoryRow) -> String {
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

impl PageSnapshot for SettingsInventorySnapshot {
    type Presenter = InventoryPagePresenter;

    /// The visible rows and the selected one, each Location's Areas, Units and insurers, and each
    /// Area's Item count.
    fn read(page: &InventoryPage, _cx: &App) -> Self {
        let routes = page.setup();
        let inventory = routes.inventory.borrow().inventory();
        let view = page.view_model();
        let units = &routes.reads;
        SettingsInventorySnapshot {
            inventory_rows: view
                .rows
                .iter()
                .map(|line| row_label(&inventory, line.row))
                .collect(),
            selected_inventory: view.selected.map(|row| row_label(&inventory, row)),
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
                    row_label(&inventory, menu.row),
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
