//! The Settings Inventory page driven by keys: one flat selection over Property and Room rows,
//! tree keys, `J`/`K` reorder, the placeholder dialog keys, the empty state and the status line.
//! State is read back from `Shell`.

mod common;

use common::Harness;
use gpui::TestAppContext;

const ELM: &str = "property:12 Elm St contents";
const STORAGE: &str = "property:Storage unit";

/// Settings on the Inventory page (nine presses down the index) with focus stepped into it.
fn in_inventory(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    ui.press("j j j j j j j j j");
    assert_eq!(ui.settings().page, "Inventory");
    ui.press("l");
    assert!(ui.settings().page_focused);
    ui
}

#[gpui::test]
fn the_page_follows_documents_and_starts_collapsed_on_the_first_property(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    let page = ui.settings();
    assert_eq!(page.inventory_rows, [ELM, STORAGE]);
    assert_eq!(page.selected_inventory.as_deref(), Some(ELM));
    ui.press("j");
    assert_eq!(ui.settings().page, "Inventory");
}

#[gpui::test]
fn the_palette_opens_the_page(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("g s");
    ui.press(": s e t t i n g s space i n v e n t o r y enter");
    assert_eq!(ui.settings().page, "Inventory");
}

#[gpui::test]
fn right_opens_a_property_then_steps_into_its_rooms(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    ui.press("right");
    let page = ui.settings();
    assert!(page.inventory_rows.len() > 2, "the Rooms are listed");
    assert_eq!(page.selected_inventory.as_deref(), Some(ELM));
    let first_room = page.inventory_rows[1].clone();
    assert!(first_room.starts_with("room:"));
    ui.press("right");
    assert_eq!(ui.settings().selected_inventory, Some(first_room));
}

#[gpui::test]
fn j_and_k_walk_properties_and_rooms_as_one_list_without_the_add_row(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    ui.press("right");
    let rows = ui.settings().inventory_rows;
    for row in rows.iter().skip(1) {
        ui.press("j");
        assert_eq!(ui.settings().selected_inventory.as_ref(), Some(row));
    }
    // The last row is the next Property: nothing, such as `+ Add room`, sits between.
    assert_eq!(rows.last().map(String::as_str), Some(STORAGE));
    ui.press("j");
    assert_eq!(ui.settings().selected_inventory.as_deref(), Some(STORAGE));
    ui.press("g g");
    assert_eq!(ui.settings().selected_inventory.as_deref(), Some(ELM));
    ui.press("shift-g");
    assert_eq!(ui.settings().selected_inventory.as_deref(), Some(STORAGE));
    ui.press("k");
    assert!(
        ui.settings()
            .selected_inventory
            .is_some_and(|row| row.starts_with("room:"))
    );
}

#[gpui::test]
fn left_climbs_from_a_room_and_then_closes_the_property(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    ui.press("right right");
    assert!(
        ui.settings()
            .selected_inventory
            .is_some_and(|row| row.starts_with("room:"))
    );
    ui.press("left");
    let page = ui.settings();
    assert_eq!(page.selected_inventory.as_deref(), Some(ELM));
    assert!(page.inventory_rows.len() > 2, "still open after climbing");
    ui.press("left");
    let page = ui.settings();
    assert_eq!(page.inventory_rows, [ELM, STORAGE]);
    assert!(page.page_focused, "closing a Property keeps focus");
    // A closed Property has nothing to fold, so left hands focus back to the index.
    ui.press("left");
    assert!(!ui.settings().page_focused);
}

#[gpui::test]
fn shift_j_and_shift_k_reorder_rooms_within_their_property(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    ui.press("right right");
    let before = ui.settings().inventory_rooms[0].1.clone();
    let moved = before[0].clone();
    ui.press("shift-j");
    let after = ui.settings().inventory_rooms[0].1.clone();
    assert_eq!(after[1], moved);
    assert_eq!(after[0], before[1]);
    ui.press("shift-k");
    assert_eq!(ui.settings().inventory_rooms[0].1, before);
    // At the top edge it is a no-op.
    ui.press("shift-k");
    assert_eq!(ui.settings().inventory_rooms[0].1, before);
    assert_eq!(
        ui.settings().selected_inventory,
        Some(format!("room:{moved}"))
    );
}

#[gpui::test]
fn shift_j_is_inert_on_a_property_row_and_at_the_last_room(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);
    let rooms = ui.settings().inventory_rooms;

    ui.press("shift-j");
    assert_eq!(ui.settings().inventory_rooms, rooms);
    assert_eq!(ui.settings().selected_inventory.as_deref(), Some(ELM));

    ui.press("right right");
    for _ in 0..rooms[0].1.len() {
        ui.press("j");
    }
    ui.press("shift-j");
    assert_eq!(ui.settings().inventory_rooms, rooms);
}

#[gpui::test]
fn n_r_e_and_x_ask_for_the_placeholder_dialogs(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    ui.press("n");
    assert_eq!(ui.settings().inventory_stub.as_deref(), Some("AddProperty"));
    ui.press("e");
    assert!(
        ui.settings()
            .inventory_stub
            .is_some_and(|stub| stub.starts_with("Edit(Property"))
    );
    ui.press("x");
    assert!(
        ui.settings()
            .inventory_stub
            .is_some_and(|stub| stub.starts_with("Remove(Property"))
    );
}

#[gpui::test]
fn r_expands_a_collapsed_property_and_works_from_a_room(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    ui.press("r");
    let page = ui.settings();
    assert!(page.inventory_rows.len() > 2, "r opens the Property");
    assert!(
        page.inventory_stub
            .is_some_and(|stub| stub.starts_with("AddRoom"))
    );
    ui.press("right right");
    ui.press("n");
    ui.press("r");
    assert_eq!(
        ui.settings()
            .selected_inventory
            .as_deref()
            .map(|r| r.starts_with("room:")),
        Some(true)
    );
    assert!(
        ui.settings()
            .inventory_stub
            .is_some_and(|stub| stub.starts_with("AddRoom"))
    );
}

#[gpui::test]
fn the_empty_state_ignores_r_e_and_x_but_r_raises_a_toast(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.empty_inventory();
    ui.press("g s");
    ui.press("j j j j j j j j j l");
    let page = ui.settings();
    assert_eq!(page.page, "Inventory");
    assert!(page.inventory_rows.is_empty());
    assert_eq!(page.selected_inventory, None);

    ui.press("e x");
    assert_eq!(ui.settings().inventory_stub, None);
    ui.press("j k shift-j right");
    assert!(ui.settings().inventory_rows.is_empty());
    ui.press("r");
    let toasts = ui.toasts();
    assert_eq!(
        toasts.visible.first().map(|toast| toast.text.as_str()),
        Some("Add a property first")
    );
    assert_eq!(ui.settings().inventory_stub, None);
    ui.press("n");
    assert_eq!(ui.settings().inventory_stub.as_deref(), Some("AddProperty"));
}

#[gpui::test]
fn the_page_has_no_effect_on_keys_while_the_index_has_focus(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    ui.press("escape");
    assert!(!ui.settings().page_focused);
    ui.press("n r e x");
    assert_eq!(ui.settings().inventory_stub, None);
    assert_eq!(ui.settings().page, "Inventory");
}
