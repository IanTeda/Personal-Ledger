//! The Settings Inventory page driven by clicks: row selection, the disclosure toggle, the
//! row actions and the Add buttons. Only elements tagged with `debug_selector` are clicked.

use super::common;
use lib_core::{PropertyId, RoomId};

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
const ELM: PropertyId = PropertyId::seed(1);
const STORAGE: PropertyId = PropertyId::seed(2);

#[gpui::test]
fn clicking_a_property_selects_it_and_focuses_the_page(app: &mut TestAppContext) {
    let mut ui = on_inventory(app);

    ui.click(&format!("settings-inventory-property-{STORAGE}"));
    let inventory_page = ui.settings_inventory();
    let page = ui.settings();
    assert_eq!(
        inventory_page.selected_inventory.as_deref(),
        Some("property:Storage unit")
    );
    assert!(page.page_focused);
}

#[gpui::test]
fn the_disclosure_toggle_opens_and_closes_without_selecting(app: &mut TestAppContext) {
    let mut ui = on_inventory(app);

    ui.click(&format!("settings-inventory-toggle-{STORAGE}"));
    let inventory_page = ui.settings_inventory();
    assert!(inventory_page.inventory_rows.len() > 2);
    assert_eq!(
        inventory_page.selected_inventory.as_deref(),
        Some("property:12 Elm St contents")
    );
    ui.click(&format!("settings-inventory-toggle-{STORAGE}"));
    assert_eq!(ui.settings_inventory().inventory_rows.len(), 2);
}

#[gpui::test]
fn clicking_a_room_selects_it_and_closing_its_property_reselects_the_property(
    app: &mut TestAppContext,
) {
    let mut ui = on_inventory(app);

    ui.click(&format!("settings-inventory-toggle-{ELM}"));
    let room = ui.settings_inventory().inventory_rows[1].clone();
    assert!(room.starts_with("room:"));
    ui.click(&format!("settings-inventory-room-{}", RoomId::seed(1)));
    assert_eq!(ui.settings_inventory().selected_inventory, Some(room));
    ui.click(&format!("settings-inventory-toggle-{ELM}"));
    assert_eq!(
        ui.settings_inventory().selected_inventory.as_deref(),
        Some("property:12 Elm St contents")
    );
}

#[gpui::test]
fn the_row_actions_select_the_row_and_ask_for_their_dialogs(app: &mut TestAppContext) {
    let mut ui = on_inventory(app);

    ui.click(&format!("settings-inventory-edit-property-{STORAGE}"));
    let inventory_page = ui.settings_inventory();
    assert_eq!(
        inventory_page.selected_inventory.as_deref(),
        Some("property:Storage unit")
    );
    assert_eq!(inventory_page.inventory_dialog.as_deref(), Some("edit"));
    ui.click("edit-property-cancel");
    assert_eq!(ui.settings_inventory().inventory_dialog, None);
    ui.click(&format!("settings-inventory-remove-property-{ELM}"));
    assert_eq!(
        ui.settings_inventory().inventory_dialog.as_deref(),
        Some("remove")
    );
    ui.click("remove-property-cancel");
    assert_eq!(ui.settings_inventory().inventory_dialog, None);

    ui.click(&format!("settings-inventory-toggle-{ELM}"));
    ui.click(&format!("settings-inventory-edit-room-{}", RoomId::seed(1)));
    assert_eq!(
        ui.settings_inventory().inventory_dialog.as_deref(),
        Some("edit-room")
    );
    ui.click("edit-room-cancel");
    assert_eq!(ui.settings_inventory().inventory_dialog, None);
    assert!(
        ui.settings_inventory()
            .selected_inventory
            .is_some_and(|row| row.starts_with("room:"))
    );
}

#[gpui::test]
fn the_add_buttons_ask_for_their_dialogs(app: &mut TestAppContext) {
    let mut ui = on_inventory(app);

    ui.click("settings-inventory-add-property");
    assert_eq!(
        ui.settings_inventory().inventory_dialog.as_deref(),
        Some("add")
    );
    assert!(ui.settings().page_focused);
    ui.click("add-property-cancel");

    // + Add room exists only under an open Property, and is not a selectable row.
    assert!(
        ui.cx
            .debug_bounds(Box::leak(
                format!("settings-inventory-add-room-{}", RoomId::seed(1)).into_boxed_str()
            ))
            .is_none()
    );
    ui.click(&format!("settings-inventory-toggle-{ELM}"));
    let selected = ui.settings_inventory().selected_inventory;
    ui.click(&format!("settings-inventory-add-room-{ELM}"));
    let inventory_page = ui.settings_inventory();
    assert_eq!(inventory_page.inventory_dialog.as_deref(), Some("add-room"));
    assert_eq!(inventory_page.selected_inventory, selected);
}

#[gpui::test]
fn the_empty_state_keeps_the_add_property_button(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.empty_inventory();
    ui.press("g s");
    ui.click("settings-index-Inventory");

    assert!(ui.settings_inventory().inventory_rows.is_empty());
    assert!(
        ui.cx
            .debug_bounds("settings-inventory-add-property")
            .is_some()
    );
    ui.click("settings-inventory-add-property");
    assert_eq!(
        ui.settings_inventory().inventory_dialog.as_deref(),
        Some("add")
    );
}

#[gpui::test]
fn the_add_dialog_submits_by_its_button_and_picks_an_insurer_suggestion(app: &mut TestAppContext) {
    let mut ui = on_inventory(app);

    ui.click("settings-inventory-add-property");
    ui.click("property-name");
    ui.press("c a b i n");
    ui.click("add-property-confirm");
    let inventory_page = ui.settings_inventory();
    assert_eq!(inventory_page.inventory_dialog, None);
    assert_eq!(
        inventory_page
            .inventory_properties
            .last()
            .map(|p| p.0.as_str()),
        Some("cabin")
    );
}

#[gpui::test]
fn the_add_dialog_fills_the_insurer_from_a_suggestion(app: &mut TestAppContext) {
    let mut ui = on_inventory(app);
    let insurer = ui
        .settings_inventory()
        .inventory_properties
        .iter()
        .find_map(|p| p.2.clone())
        .expect("the seed has an insured Property");

    ui.click("settings-inventory-add-property");
    ui.click("property-name");
    ui.press("c a b i n");
    ui.click("property-insurer");
    ui.click("property-insurer-suggestion-0");
    ui.click("property-sum-insured");
    ui.press("5 0 0");
    ui.click("add-property-confirm");
    assert_eq!(
        ui.settings_inventory()
            .inventory_properties
            .last()
            .and_then(|p| p.2.clone()),
        Some(insurer)
    );
}

#[gpui::test]
fn the_remove_dialog_confirms_by_its_button(app: &mut TestAppContext) {
    let mut ui = on_inventory(app);

    ui.click("settings-inventory-add-property");
    ui.click("property-name");
    ui.press("c a b i n");
    ui.click("add-property-confirm");
    let cabin = ui
        .settings_inventory()
        .inventory_property_ids
        .last()
        .copied()
        .expect("the added Property");
    ui.click(&format!("settings-inventory-remove-property-{cabin}"));
    ui.click("remove-property-confirm");
    assert_eq!(ui.settings_inventory().inventory_properties.len(), 2);
    assert_eq!(ui.settings_inventory().inventory_dialog, None);
}

#[gpui::test]
fn add_room_submits_by_its_button_under_the_clicked_property(app: &mut TestAppContext) {
    let mut ui = on_inventory(app);
    ui.click(&format!("settings-inventory-toggle-{STORAGE}"));
    let before = ui.settings_inventory().inventory_rooms[1].1.len();

    ui.click(&format!("settings-inventory-add-room-{STORAGE}"));
    ui.click("room-name");
    ui.press("l o f t");
    ui.click("add-room-confirm");
    let inventory_page = ui.settings_inventory();
    assert_eq!(inventory_page.inventory_dialog, None);
    assert_eq!(inventory_page.inventory_rooms[1].1.len(), before + 1);
    assert_eq!(
        inventory_page.inventory_rooms[1]
            .1
            .last()
            .map(String::as_str),
        Some("loft")
    );
    assert_eq!(
        inventory_page.selected_inventory.as_deref(),
        Some("room:loft")
    );
}

#[gpui::test]
fn a_property_row_has_its_own_add_room_button(app: &mut TestAppContext) {
    let mut ui = on_inventory(app);
    // Collapsed: the + room button on the Property row still opens the dialog (16q).
    let selected = ui.settings_inventory().selected_inventory;
    ui.click(&format!("settings-inventory-property-add-room-{STORAGE}"));
    let inventory_page = ui.settings_inventory();
    assert_eq!(inventory_page.inventory_dialog.as_deref(), Some("add-room"));
    assert_eq!(inventory_page.selected_inventory, selected);

    ui.click("room-name");
    ui.press("l o f t");
    ui.click("add-room-confirm");
    let inventory_page = ui.settings_inventory();
    assert_eq!(inventory_page.inventory_dialog, None);
    assert_eq!(
        inventory_page.inventory_rooms[1]
            .1
            .last()
            .map(String::as_str),
        Some("loft")
    );
}

#[gpui::test]
fn edit_room_saves_by_its_button(app: &mut TestAppContext) {
    let mut ui = on_inventory(app);
    ui.click(&format!("settings-inventory-toggle-{ELM}"));

    ui.click(&format!("settings-inventory-edit-room-{}", RoomId::seed(1)));
    ui.click("room-name");
    ui.press("s");
    ui.click("edit-room-confirm");
    assert_eq!(ui.settings_inventory().inventory_dialog, None);
    assert!(ui.settings_inventory().inventory_rooms[0].1[0].ends_with('s'));
}

#[gpui::test]
fn remove_room_moves_items_by_the_picker_and_the_confirm_button(app: &mut TestAppContext) {
    let mut ui = on_inventory(app);
    ui.click(&format!("settings-inventory-toggle-{ELM}"));
    let rooms = ui.settings_inventory().inventory_rooms[0].1.clone();
    let count = |ui: &mut Harness<'_>, name: &str| {
        ui.settings_inventory()
            .inventory_room_items
            .iter()
            .find(|(room, _)| room == name)
            .map_or(0, |(_, items)| *items)
    };
    let moving = count(&mut ui, &rooms[0]);
    let third = count(&mut ui, &rooms[2]);

    ui.click(&format!(
        "settings-inventory-remove-room-{}",
        RoomId::seed(1)
    ));
    assert_eq!(
        ui.settings_inventory().inventory_dialog.as_deref(),
        Some("remove-room")
    );
    ui.click("room-destination");
    ui.click("room-destination-option-1");
    ui.click("remove-room-confirm");
    assert_eq!(ui.settings_inventory().inventory_dialog, None);
    assert_eq!(count(&mut ui, &rooms[2]), third + moving);
    assert_eq!(
        ui.settings_inventory().inventory_rooms[0].1.len(),
        rooms.len() - 1
    );
}

#[gpui::test]
fn the_last_room_with_items_shows_the_notice_which_closes_by_its_button(app: &mut TestAppContext) {
    let mut ui = on_inventory(app);
    ui.click(&format!("settings-inventory-toggle-{STORAGE}"));

    ui.click(&format!(
        "settings-inventory-remove-room-{}",
        RoomId::seed(11)
    ));
    assert_eq!(
        ui.settings_inventory().inventory_dialog.as_deref(),
        Some("room-blocked")
    );
    ui.click("room-blocked-close");
    assert_eq!(ui.settings_inventory().inventory_dialog, None);
    assert_eq!(ui.settings_inventory().inventory_rooms[1].1.len(), 1);
}
