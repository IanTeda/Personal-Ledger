//! The Settings Inventory page driven by clicks: row selection, the disclosure toggle, the
//! row actions and the Add buttons. Only elements tagged with `debug_selector` are clicked.

mod common;

use common::Harness;
use gpui::TestAppContext;

fn on_inventory(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    ui.click("settings-index-Inventory");
    assert_eq!(ui.settings().page, "Inventory");
    ui
}

/// The first Property's id and the second's, from the seed's creation order.
const ELM: u32 = 1;
const STORAGE: u32 = 2;

#[gpui::test]
fn clicking_a_property_selects_it_and_focuses_the_page(app: &mut TestAppContext) {
    let mut ui = on_inventory(app);

    ui.click(&format!("settings-inventory-property-{STORAGE}"));
    let page = ui.settings();
    assert_eq!(
        page.selected_inventory.as_deref(),
        Some("property:Storage unit")
    );
    assert!(page.page_focused);
}

#[gpui::test]
fn the_disclosure_toggle_opens_and_closes_without_selecting(app: &mut TestAppContext) {
    let mut ui = on_inventory(app);

    ui.click(&format!("settings-inventory-toggle-{STORAGE}"));
    let page = ui.settings();
    assert!(page.inventory_rows.len() > 2);
    assert_eq!(
        page.selected_inventory.as_deref(),
        Some("property:12 Elm St contents")
    );
    ui.click(&format!("settings-inventory-toggle-{STORAGE}"));
    assert_eq!(ui.settings().inventory_rows.len(), 2);
}

#[gpui::test]
fn clicking_a_room_selects_it_and_closing_its_property_reselects_the_property(
    app: &mut TestAppContext,
) {
    let mut ui = on_inventory(app);

    ui.click(&format!("settings-inventory-toggle-{ELM}"));
    let room = ui.settings().inventory_rows[1].clone();
    assert!(room.starts_with("room:"));
    ui.click("settings-inventory-room-1");
    assert_eq!(ui.settings().selected_inventory, Some(room));
    ui.click(&format!("settings-inventory-toggle-{ELM}"));
    assert_eq!(
        ui.settings().selected_inventory.as_deref(),
        Some("property:12 Elm St contents")
    );
}

#[gpui::test]
fn the_row_actions_select_the_row_and_ask_for_their_dialogs(app: &mut TestAppContext) {
    let mut ui = on_inventory(app);

    ui.click(&format!("settings-inventory-edit-property-{STORAGE}"));
    let page = ui.settings();
    assert_eq!(
        page.selected_inventory.as_deref(),
        Some("property:Storage unit")
    );
    assert_eq!(
        page.inventory_stub,
        Some(format!("Edit(Property({STORAGE}))"))
    );
    ui.click(&format!("settings-inventory-remove-property-{ELM}"));
    assert_eq!(
        ui.settings().inventory_stub,
        Some(format!("Remove(Property({ELM}))"))
    );

    ui.click(&format!("settings-inventory-toggle-{ELM}"));
    ui.click("settings-inventory-edit-room-1");
    assert_eq!(
        ui.settings().inventory_stub.as_deref(),
        Some("Edit(Room(1))")
    );
    assert!(
        ui.settings()
            .selected_inventory
            .is_some_and(|row| row.starts_with("room:"))
    );
}

#[gpui::test]
fn the_add_buttons_ask_for_their_dialogs(app: &mut TestAppContext) {
    let mut ui = on_inventory(app);

    ui.click("settings-inventory-add-property");
    assert_eq!(ui.settings().inventory_stub.as_deref(), Some("AddProperty"));
    assert!(ui.settings().page_focused);

    // + Add room exists only under an open Property, and is not a selectable row.
    assert!(
        ui.cx
            .debug_bounds("settings-inventory-add-room-1")
            .is_none()
    );
    ui.click(&format!("settings-inventory-toggle-{ELM}"));
    let selected = ui.settings().selected_inventory;
    ui.click(&format!("settings-inventory-add-room-{ELM}"));
    let page = ui.settings();
    assert_eq!(page.inventory_stub, Some(format!("AddRoom({ELM})")));
    assert_eq!(page.selected_inventory, selected);
}

#[gpui::test]
fn the_empty_state_keeps_the_add_property_button(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.empty_inventory();
    ui.press("g s");
    ui.click("settings-index-Inventory");

    assert!(ui.settings().inventory_rows.is_empty());
    assert!(
        ui.cx
            .debug_bounds("settings-inventory-add-property")
            .is_some()
    );
    ui.click("settings-inventory-add-property");
    assert_eq!(ui.settings().inventory_stub.as_deref(), Some("AddProperty"));
}
