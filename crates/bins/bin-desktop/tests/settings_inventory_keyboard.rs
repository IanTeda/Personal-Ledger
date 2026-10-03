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
fn n_e_and_x_open_the_property_dialogs_and_escape_cancels(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    ui.press("n");
    assert_eq!(ui.settings().inventory_dialog.as_deref(), Some("add"));
    ui.press("escape");
    assert_eq!(ui.settings().inventory_dialog, None);
    assert!(ui.settings().page_focused, "focus returns to the page");
    ui.press("e");
    assert_eq!(ui.settings().inventory_dialog.as_deref(), Some("edit"));
    ui.press("escape x");
    assert_eq!(ui.settings().inventory_dialog.as_deref(), Some("remove"));
    ui.press("escape");
    assert_eq!(ui.settings().inventory_dialog, None);
    ui.press("right right e");
    assert_eq!(ui.settings().inventory_dialog.as_deref(), Some("edit-room"));
}

fn names(ui: &mut Harness<'_>) -> Vec<String> {
    ui.settings()
        .inventory_properties
        .into_iter()
        .map(|(name, ..)| name)
        .collect()
}

#[gpui::test]
fn add_needs_a_name_then_adds_expands_selects_and_toasts(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    ui.press("n enter");
    assert_eq!(
        ui.settings().inventory_dialog.as_deref(),
        Some("add"),
        "enter is inert with no name"
    );
    ui.press("c a b i n enter");
    let page = ui.settings();
    assert_eq!(page.inventory_dialog, None);
    let added = page.inventory_properties.last().cloned().unwrap();
    assert_eq!(added, ("cabin".to_string(), "aud".to_string(), None));
    assert_eq!(page.selected_inventory.as_deref(), Some("property:cabin"));
    assert!(page.inventory_rows.contains(&"property:cabin".to_string()));
    assert_eq!(
        ui.toasts().visible.first().map(|t| t.text.clone()),
        Some("Added property \"cabin\"".to_string())
    );
}

#[gpui::test]
fn add_refuses_a_name_another_property_has_ignoring_case(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    ui.press("n s t o r a g e space u n i t enter");
    assert_eq!(ui.settings().inventory_dialog.as_deref(), Some("add"));
    assert_eq!(names(&mut ui).len(), 2);
}

#[gpui::test]
fn an_insurer_needs_a_sum_and_emptying_it_means_no_cover(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    // Name, Address, Unit, Insurer.
    ui.press("n c a b i n tab tab tab n r m a enter");
    assert_eq!(ui.settings().inventory_dialog.as_deref(), Some("add"));
    // Policy number, Renews on, Sum insured.
    ui.press("tab tab tab 1 0 0 0 enter");
    let page = ui.settings();
    assert_eq!(page.inventory_dialog, None);
    assert_eq!(
        page.inventory_properties.last().and_then(|p| p.2.clone()),
        Some("nrma".to_string())
    );

    // Back on Insurer, backspacing it away clears the cover and the sum it needed.
    ui.press("n c a b 2 tab tab tab n backspace enter");
    assert_eq!(
        ui.settings()
            .inventory_properties
            .last()
            .map(|p| p.2.clone()),
        Some(None)
    );
}

#[gpui::test]
fn edit_saves_the_name_and_keeps_the_unit(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);
    let before = ui.settings().inventory_properties[0].clone();

    ui.press("e x enter");
    let after = ui.settings().inventory_properties[0].clone();
    assert_eq!(after.0, format!("{}x", before.0));
    assert_eq!(after.1, before.1);
    assert_eq!(ui.settings().inventory_dialog, None);
    assert_eq!(
        ui.toasts().visible.first().map(|t| t.text.clone()),
        Some(format!("Saved property \"{}x\"", before.0))
    );
}

#[gpui::test]
fn removing_an_empty_property_is_a_plain_confirm(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    ui.press("n c a b i n enter x enter");
    assert_eq!(ui.settings().inventory_dialog, None);
    assert_eq!(names(&mut ui).len(), 2);
    assert!(
        ui.toasts()
            .visible
            .iter()
            .any(|t| t.text == "Removed property \"cabin\"")
    );
    assert!(ui.settings().selected_inventory.is_some());
}

#[gpui::test]
fn removing_a_property_with_contents_asks_for_its_name_typed(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    ui.press("x enter");
    assert_eq!(
        ui.settings().inventory_dialog.as_deref(),
        Some("remove"),
        "enter is inert until the name is typed"
    );
    ui.press("1 2 space shift-e l m space shift-s t space c o n t e n t s enter");
    let page = ui.settings();
    assert_eq!(page.inventory_dialog, None);
    assert_eq!(
        page.inventory_properties
            .iter()
            .map(|p| p.0.as_str())
            .collect::<Vec<_>>(),
        ["Storage unit"]
    );
    assert_eq!(
        page.selected_inventory.as_deref(),
        Some("property:Storage unit")
    );
}

#[gpui::test]
fn the_last_property_can_be_removed(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.empty_inventory();
    ui.press("g s");
    ui.press("j j j j j j j j j l n o n e enter");
    assert_eq!(names(&mut ui), vec!["one".to_string()]);
    ui.press("x enter");
    assert!(names(&mut ui).is_empty());
    assert_eq!(ui.settings().selected_inventory, None);
}

#[gpui::test]
fn r_expands_a_collapsed_property_and_works_from_a_room(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    ui.press("r");
    let page = ui.settings();
    assert!(page.inventory_rows.len() > 2, "r opens the Property");
    assert_eq!(page.inventory_dialog.as_deref(), Some("add-room"));
    ui.press("escape right right");
    ui.press("r");
    assert_eq!(
        ui.settings()
            .selected_inventory
            .as_deref()
            .map(|r| r.starts_with("room:")),
        Some(true)
    );
    assert_eq!(ui.settings().inventory_dialog.as_deref(), Some("add-room"));
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
    assert_eq!(ui.settings().inventory_dialog, None);
    ui.press("j k shift-j right");
    assert!(ui.settings().inventory_rows.is_empty());
    ui.press("r");
    let toasts = ui.toasts();
    assert_eq!(
        toasts.visible.first().map(|toast| toast.text.as_str()),
        Some("Add a property first")
    );
    ui.press("n");
    assert_eq!(ui.settings().inventory_dialog.as_deref(), Some("add"));
}

#[gpui::test]
fn the_page_has_no_effect_on_keys_while_the_index_has_focus(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    ui.press("escape");
    assert!(!ui.settings().page_focused);
    ui.press("n r e x");
    assert_eq!(ui.settings().inventory_dialog, None);
    assert_eq!(ui.settings().page, "Inventory");
}

/// 12 Elm St opened, its first Room selected.
fn on_first_room(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = in_inventory(app);
    ui.press("right right");
    assert!(
        ui.settings()
            .selected_inventory
            .is_some_and(|r| r.starts_with("room:"))
    );
    ui
}

fn room_names(ui: &mut Harness<'_>, property: usize) -> Vec<String> {
    ui.settings().inventory_rooms[property].1.clone()
}

fn room_items(ui: &mut Harness<'_>, name: &str) -> usize {
    ui.settings()
        .inventory_room_items
        .iter()
        .find(|(room, _)| room == name)
        .map_or(0, |(_, items)| *items)
}

#[gpui::test]
fn add_room_appends_it_last_selects_it_and_updates_the_count(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);
    let before = room_names(&mut ui, 0).len();

    ui.press("r");
    assert_eq!(ui.settings().inventory_dialog.as_deref(), Some("add-room"));
    ui.press("enter");
    assert_eq!(
        ui.settings().inventory_dialog.as_deref(),
        Some("add-room"),
        "an empty name keeps the dialog open"
    );
    ui.press("a t t i c enter");
    let page = ui.settings();
    assert_eq!(page.inventory_dialog, None);
    assert!(page.page_focused, "focus returns to the page");
    assert_eq!(room_names(&mut ui, 0).len(), before + 1);
    assert_eq!(
        room_names(&mut ui, 0).last().map(String::as_str),
        Some("attic")
    );
    assert_eq!(
        ui.settings().selected_inventory.as_deref(),
        Some("room:attic")
    );
    assert_eq!(room_items(&mut ui, "attic"), 0);
    let toasts = ui.toasts();
    assert_eq!(
        toasts.visible.first().map(|toast| toast.text.as_str()),
        Some("Added room \"attic\"")
    );
}

#[gpui::test]
fn a_room_name_may_repeat_in_another_property_but_not_in_its_own(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);
    let living = room_names(&mut ui, 0)[0].clone();

    ui.press("r");
    for ch in living.to_uppercase().chars() {
        ui.press(&ch.to_string());
    }
    ui.press("enter");
    assert_eq!(
        ui.settings().inventory_dialog.as_deref(),
        Some("add-room"),
        "the same name, ignoring case, is refused in its own Property"
    );
    ui.press("escape");

    // The Storage unit is another Property, so the name is free there.
    ui.press("left j r");
    for ch in living.chars() {
        ui.press(&ch.to_string());
    }
    ui.press("enter");
    assert_eq!(ui.settings().inventory_dialog, None);
    assert!(room_names(&mut ui, 1).contains(&living));
}

#[gpui::test]
fn edit_room_renames_it_and_keeps_its_items_and_position(app: &mut TestAppContext) {
    let mut ui = on_first_room(app);
    let before = room_names(&mut ui, 0);
    let items = room_items(&mut ui, &before[0]);

    ui.press("e");
    assert_eq!(ui.settings().inventory_dialog.as_deref(), Some("edit-room"));
    ui.press("escape");
    assert_eq!(ui.settings().inventory_dialog, None);
    ui.press("e backspace x enter");
    let after = room_names(&mut ui, 0);
    assert_eq!(after.len(), before.len());
    assert_eq!(after[0], format!("{}x", &before[0][..before[0].len() - 1]));
    assert_eq!(room_items(&mut ui, &after[0]), items);
    assert_eq!(&after[1..], &before[1..]);
}

#[gpui::test]
fn removing_an_empty_room_is_a_plain_confirm(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);
    ui.press("r a t t i c enter");
    let with_attic = room_names(&mut ui, 0);

    ui.press("x");
    assert_eq!(
        ui.settings().inventory_dialog.as_deref(),
        Some("remove-room")
    );
    ui.press("escape");
    assert_eq!(room_names(&mut ui, 0), with_attic);
    ui.press("x enter");
    let page = ui.settings();
    assert_eq!(page.inventory_dialog, None);
    assert_eq!(room_names(&mut ui, 0).len(), with_attic.len() - 1);
    let toasts = ui.toasts();
    assert!(
        toasts
            .visible
            .iter()
            .any(|toast| toast.text == "Removed room \"attic\"")
    );
    assert!(
        ui.settings()
            .selected_inventory
            .is_some_and(|r| r.starts_with("room:"))
    );
}

#[gpui::test]
fn removing_a_room_with_items_moves_them_to_the_room_above_by_default(app: &mut TestAppContext) {
    let mut ui = on_first_room(app);
    ui.press("j");
    let rooms = room_names(&mut ui, 0);
    let (second, first) = (rooms[1].clone(), rooms[0].clone());
    let moving = room_items(&mut ui, &second);
    let first_before = room_items(&mut ui, &first);
    let total = ui
        .settings()
        .inventory_room_items
        .iter()
        .map(|r| r.1)
        .sum::<usize>();

    ui.press("x");
    assert_eq!(
        ui.settings().inventory_dialog.as_deref(),
        Some("remove-room")
    );
    ui.press("enter");
    assert_eq!(ui.settings().inventory_dialog, None);
    assert!(!room_names(&mut ui, 0).contains(&second));
    assert_eq!(room_items(&mut ui, &first), first_before + moving);
    assert_eq!(
        ui.settings()
            .inventory_room_items
            .iter()
            .map(|r| r.1)
            .sum::<usize>(),
        total,
        "no Item is lost"
    );
}

#[gpui::test]
fn the_first_room_pre_selects_the_room_below_and_the_picker_can_change_it(
    app: &mut TestAppContext,
) {
    let mut ui = on_first_room(app);
    let rooms = room_names(&mut ui, 0);
    let moving = room_items(&mut ui, &rooms[0]);
    let second_before = room_items(&mut ui, &rooms[1]);
    let third_before = room_items(&mut ui, &rooms[2]);

    ui.press("x space down enter");
    assert_eq!(
        ui.settings().inventory_dialog.as_deref(),
        Some("remove-room"),
        "enter on the open list picks, it does not confirm"
    );
    ui.press("enter");
    assert_eq!(room_items(&mut ui, &rooms[1]), second_before);
    assert_eq!(room_items(&mut ui, &rooms[2]), third_before + moving);
}

#[gpui::test]
fn the_last_room_with_items_is_blocked_and_nothing_changes(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);
    // The Storage unit has one Room, holding Items.
    ui.press("j right right x");
    assert_eq!(
        ui.settings().inventory_dialog.as_deref(),
        Some("room-blocked")
    );
    ui.press("enter");
    assert_eq!(ui.settings().inventory_dialog, None);
    assert_eq!(room_names(&mut ui, 1).len(), 1);
    ui.press("x escape");
    assert_eq!(ui.settings().inventory_dialog, None);
    assert!(ui.settings().page_focused);
}
