//! The Settings Inventory page driven by the mouse (16q): row selection, the disclosure toggle,
//! double-click to edit, the right-click row menus and the square Add location button. Only
//! elements tagged with `debug_selector` are clicked.

use super::common;
use bin_desktop::shell::router::SettingsInventorySnapshot;
use lib_core::{AreaId, LocationId};

use common::Harness;
use gpui::TestAppContext;

fn on_inventory(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    ui.click("settings-index-Inventory");
    assert_eq!(ui.settings_frame().page, "Inventory");
    ui
}

/// The seed's Locations' ids, in creation order.
const ELM: LocationId = LocationId::seed(1);
const STORAGE: LocationId = LocationId::seed(2);
const HILUX: LocationId = LocationId::seed(3);

fn location_row(id: LocationId) -> String {
    format!("settings-inventory-location-{id}")
}

fn area_row(id: AreaId) -> String {
    format!("settings-inventory-area-{id}")
}

/// The open row menu's row and entries, as the snapshot reads them.
fn menu(ui: &mut Harness<'_>) -> Option<(String, Vec<String>)> {
    ui.settings::<SettingsInventorySnapshot>().inventory_menu
}

fn dialog(ui: &mut Harness<'_>) -> Option<String> {
    ui.dialog().map(str::to_string)
}

#[gpui::test]
fn clicking_a_location_selects_it_and_focuses_the_page(app: &mut TestAppContext) {
    let mut ui = on_inventory(app);

    ui.click(&format!("settings-inventory-location-{STORAGE}"));
    let inventory_page = ui.settings::<SettingsInventorySnapshot>();
    let page = ui.settings_frame();
    assert_eq!(
        inventory_page.selected_inventory.as_deref(),
        Some("location:Storage unit")
    );
    assert!(page.page_focused);
}

#[gpui::test]
fn the_disclosure_toggle_opens_and_closes_without_selecting(app: &mut TestAppContext) {
    let mut ui = on_inventory(app);

    ui.click(&format!("settings-inventory-toggle-{STORAGE}"));
    let inventory_page = ui.settings::<SettingsInventorySnapshot>();
    assert!(inventory_page.inventory_rows.len() > 2);
    assert_eq!(
        inventory_page.selected_inventory.as_deref(),
        Some("location:12 Elm St contents")
    );
    ui.click(&format!("settings-inventory-toggle-{STORAGE}"));
    assert_eq!(
        ui.settings::<SettingsInventorySnapshot>()
            .inventory_rows
            .len(),
        3
    );
}

#[gpui::test]
fn clicking_an_area_selects_it_and_closing_its_location_reselects_the_location(
    app: &mut TestAppContext,
) {
    let mut ui = on_inventory(app);

    ui.click(&format!("settings-inventory-toggle-{ELM}"));
    let area = ui.settings::<SettingsInventorySnapshot>().inventory_rows[1].clone();
    assert!(area.starts_with("area:"));
    ui.click(&format!("settings-inventory-area-{}", AreaId::seed(1)));
    assert_eq!(
        ui.settings::<SettingsInventorySnapshot>()
            .selected_inventory,
        Some(area)
    );
    ui.click(&format!("settings-inventory-toggle-{ELM}"));
    assert_eq!(
        ui.settings::<SettingsInventorySnapshot>()
            .selected_inventory
            .as_deref(),
        Some("location:12 Elm St contents")
    );
}

#[gpui::test]
fn double_clicking_a_row_opens_its_edit_dialog(app: &mut TestAppContext) {
    let mut ui = on_inventory(app);

    ui.double_click(&location_row(STORAGE));
    assert_eq!(
        ui.settings::<SettingsInventorySnapshot>()
            .selected_inventory
            .as_deref(),
        Some("location:Storage unit")
    );
    assert_eq!(dialog(&mut ui).as_deref(), Some("EditLocation"));
    ui.click("edit-location-cancel");
    assert_eq!(
        ui.settings::<SettingsInventorySnapshot>()
            .inventory_rows
            .len(),
        3,
        "a double-click doesn't expand: only the disclosure does"
    );

    ui.click(&format!("settings-inventory-toggle-{ELM}"));
    ui.double_click(&area_row(AreaId::seed(1)));
    assert_eq!(dialog(&mut ui).as_deref(), Some("EditArea"));
    ui.click("edit-area-cancel");
    assert_eq!(dialog(&mut ui), None);
}

#[gpui::test]
fn right_clicking_a_location_opens_its_menu_and_escape_closes_it(app: &mut TestAppContext) {
    let mut ui = on_inventory(app);

    ui.right_click(&location_row(HILUX));
    assert_eq!(
        menu(&mut ui),
        Some((
            "location:Hilux".to_string(),
            vec![
                "add-area".to_string(),
                "edit-location".to_string(),
                "remove-location".to_string()
            ]
        ))
    );
    assert_eq!(
        ui.settings::<SettingsInventorySnapshot>()
            .selected_inventory
            .as_deref(),
        Some("location:Hilux")
    );
    assert!(ui.settings_frame().page_focused);
    ui.press("escape");
    assert_eq!(menu(&mut ui), None);
    assert!(
        ui.settings_frame().page_focused,
        "the first Esc only closes the menu"
    );
}

#[gpui::test]
fn a_press_outside_the_menu_closes_it(app: &mut TestAppContext) {
    let mut ui = on_inventory(app);

    ui.right_click(&location_row(HILUX));
    assert!(menu(&mut ui).is_some());
    ui.click(&location_row(ELM));
    assert_eq!(menu(&mut ui), None);
    assert_eq!(
        ui.settings::<SettingsInventorySnapshot>()
            .selected_inventory
            .as_deref(),
        Some("location:12 Elm St contents")
    );
}

#[gpui::test]
fn the_location_menu_asks_for_its_dialogs(app: &mut TestAppContext) {
    let mut ui = on_inventory(app);

    ui.right_click(&location_row(STORAGE));
    ui.click("settings-inventory-menu-edit-location");
    assert_eq!(menu(&mut ui), None);
    assert_eq!(dialog(&mut ui).as_deref(), Some("EditLocation"));
    ui.click("edit-location-cancel");

    ui.right_click(&location_row(ELM));
    ui.click("settings-inventory-menu-remove-location");
    assert_eq!(dialog(&mut ui).as_deref(), Some("RemoveLocation"));
    ui.click("remove-location-cancel");
    assert_eq!(dialog(&mut ui), None);
}

#[gpui::test]
fn add_area_from_a_collapsed_location_expands_it_only_once_the_area_lands(
    app: &mut TestAppContext,
) {
    let mut ui = on_inventory(app);
    let before = ui.settings::<SettingsInventorySnapshot>().inventory_areas[1]
        .1
        .len();

    ui.right_click(&location_row(STORAGE));
    ui.click("settings-inventory-menu-add-area");
    assert_eq!(dialog(&mut ui).as_deref(), Some("AddArea"));
    assert!(
        !ui.settings::<SettingsInventorySnapshot>()
            .inventory_rows
            .contains(&"area:Storage".to_string()),
        "an open Dialog has not expanded the Location"
    );
    ui.click("area-name");
    ui.press("l o f t");
    ui.click("add-area-confirm");
    let inventory_page = ui.settings::<SettingsInventorySnapshot>();
    assert_eq!(ui.dialog(), None);
    assert!(
        inventory_page
            .inventory_rows
            .contains(&"area:Storage".to_string()),
        "adding the Area expands the Location"
    );
    assert_eq!(inventory_page.inventory_areas[1].1.len(), before + 1);
    assert_eq!(
        inventory_page.selected_inventory.as_deref(),
        Some("area:loft")
    );
}

#[gpui::test]
fn the_area_menu_moves_edits_and_removes(app: &mut TestAppContext) {
    let mut ui = on_inventory(app);
    ui.click(&format!("settings-inventory-toggle-{HILUX}"));
    let names = |ui: &mut Harness<'_>| {
        ui.settings::<SettingsInventorySnapshot>().inventory_areas[2]
            .1
            .clone()
    };
    assert_eq!(names(&mut ui), ["Cab", "Tray", "Toolbox"]);
    let tray = AreaId::seed(13);

    ui.right_click(&area_row(tray));
    assert_eq!(
        menu(&mut ui).map(|(_, entries)| entries),
        Some(vec![
            "edit-area".to_string(),
            "move-up".to_string(),
            "move-down".to_string(),
            "remove-area".to_string()
        ])
    );
    ui.click("settings-inventory-menu-move-down");
    assert_eq!(names(&mut ui), ["Cab", "Toolbox", "Tray"]);
    ui.right_click(&area_row(tray));
    ui.click("settings-inventory-menu-move-up");
    ui.right_click(&area_row(tray));
    ui.click("settings-inventory-menu-move-up");
    assert_eq!(names(&mut ui), ["Tray", "Cab", "Toolbox"]);

    ui.right_click(&area_row(tray));
    ui.click("settings-inventory-menu-edit-area");
    ui.click("area-name");
    ui.press("s");
    ui.click("edit-area-confirm");
    assert_eq!(names(&mut ui), ["Trays", "Cab", "Toolbox"]);

    ui.right_click(&area_row(tray));
    ui.click("settings-inventory-menu-remove-area");
    assert_eq!(dialog(&mut ui).as_deref(), Some("RemoveArea"));
}

#[gpui::test]
fn the_add_location_button_asks_for_its_dialog(app: &mut TestAppContext) {
    let mut ui = on_inventory(app);

    ui.click("settings-inventory-add-location");
    assert_eq!(dialog(&mut ui).as_deref(), Some("AddLocation"));
    assert!(ui.settings_frame().page_focused);
    ui.click("add-location-cancel");
    assert_eq!(dialog(&mut ui), None);
}

#[gpui::test]
fn the_empty_state_keeps_the_add_location_button(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.empty_inventory();
    ui.press("g s");
    ui.click("settings-index-Inventory");

    assert!(
        ui.settings::<SettingsInventorySnapshot>()
            .inventory_rows
            .is_empty()
    );
    assert!(ui.is_drawn("settings-inventory-add-location"));
    ui.click("settings-inventory-add-location");
    assert_eq!(dialog(&mut ui).as_deref(), Some("AddLocation"));
}

#[gpui::test]
fn the_add_dialog_asks_only_for_the_name_and_unit(app: &mut TestAppContext) {
    let mut ui = on_inventory(app);

    ui.click("settings-inventory-add-location");
    assert!(ui.is_drawn("location-name"));
    assert!(ui.is_drawn("location-unit"));
    assert!(!ui.is_drawn("location-address"));
    assert!(!ui.is_drawn("location-insurer"));
    ui.click("location-name");
    ui.press("c a b i n");
    ui.click("add-location-confirm");
    let inventory_page = ui.settings::<SettingsInventorySnapshot>();
    assert_eq!(ui.dialog(), None);
    assert_eq!(
        inventory_page.inventory_locations.last().cloned(),
        Some(("cabin".to_string(), "aud".to_string(), None))
    );
}

#[gpui::test]
fn the_remove_dialog_confirms_by_its_button(app: &mut TestAppContext) {
    let mut ui = on_inventory(app);

    ui.click("settings-inventory-add-location");
    ui.click("location-name");
    ui.press("c a b i n");
    ui.click("add-location-confirm");
    let cabin = ui
        .settings::<SettingsInventorySnapshot>()
        .inventory_location_ids
        .last()
        .copied()
        .expect("the added Location");
    ui.right_click(&location_row(cabin));
    ui.click("settings-inventory-menu-remove-location");
    ui.click("remove-location-confirm");
    assert_eq!(
        ui.settings::<SettingsInventorySnapshot>()
            .inventory_locations
            .len(),
        3
    );
    assert_eq!(dialog(&mut ui), None);
}

#[gpui::test]
fn remove_area_moves_items_by_the_picker_and_the_confirm_button(app: &mut TestAppContext) {
    let mut ui = on_inventory(app);
    ui.click(&format!("settings-inventory-toggle-{ELM}"));
    let areas = ui.settings::<SettingsInventorySnapshot>().inventory_areas[0]
        .1
        .clone();
    let count = |ui: &mut Harness<'_>, name: &str| {
        ui.settings::<SettingsInventorySnapshot>()
            .inventory_area_items
            .iter()
            .find(|(area, _)| area == name)
            .map_or(0, |(_, items)| *items)
    };
    let moving = count(&mut ui, &areas[0]);
    let third = count(&mut ui, &areas[2]);

    ui.right_click(&area_row(AreaId::seed(1)));
    ui.click("settings-inventory-menu-remove-area");
    assert_eq!(dialog(&mut ui).as_deref(), Some("RemoveArea"));
    ui.click("area-destination");
    ui.click("area-destination-option-1");
    ui.click("remove-area-confirm");
    assert_eq!(dialog(&mut ui), None);
    assert_eq!(count(&mut ui, &areas[2]), third + moving);
    assert_eq!(
        ui.settings::<SettingsInventorySnapshot>().inventory_areas[0]
            .1
            .len(),
        areas.len() - 1
    );
}

#[gpui::test]
fn the_last_area_with_items_shows_the_notice_which_closes_by_its_button(app: &mut TestAppContext) {
    let mut ui = on_inventory(app);
    ui.click(&format!("settings-inventory-toggle-{STORAGE}"));

    ui.right_click(&area_row(AreaId::seed(11)));
    ui.click("settings-inventory-menu-remove-area");
    assert_eq!(dialog(&mut ui).as_deref(), Some("AreaBlocked"));
    ui.click("area-blocked-close");
    assert_eq!(dialog(&mut ui), None);
    assert_eq!(
        ui.settings::<SettingsInventorySnapshot>().inventory_areas[1]
            .1
            .len(),
        1
    );
}
