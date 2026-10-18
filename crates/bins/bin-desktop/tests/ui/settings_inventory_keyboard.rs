//! The Settings Inventory page driven by keys: one flat selection over Location and Area rows,
//! tree keys, `J`/`K` reorder, the placeholder dialog keys, the empty state and the status line.
//! State is read back from `Shell`.

use super::common;

use common::Harness;
use gpui::TestAppContext;

const ELM: &str = "location:12 Elm St contents";
const STORAGE: &str = "location:Storage unit";
const HILUX: &str = "location:Hilux";

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
fn the_page_follows_documents_and_starts_collapsed_on_the_first_location(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    let inventory_page = ui.settings_inventory();
    assert_eq!(inventory_page.inventory_rows, [ELM, STORAGE, HILUX]);
    assert_eq!(inventory_page.selected_inventory.as_deref(), Some(ELM));
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
fn right_opens_a_location_then_steps_into_its_areas(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    ui.press("right");
    let inventory_page = ui.settings_inventory();
    assert!(
        inventory_page.inventory_rows.len() > 2,
        "the Areas are listed"
    );
    assert_eq!(inventory_page.selected_inventory.as_deref(), Some(ELM));
    let first_area = inventory_page.inventory_rows[1].clone();
    assert!(first_area.starts_with("area:"));
    ui.press("right");
    assert_eq!(ui.settings_inventory().selected_inventory, Some(first_area));
}

#[gpui::test]
fn j_and_k_walk_locations_and_areas_as_one_list_without_the_add_row(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    ui.press("right");
    let rows = ui.settings_inventory().inventory_rows;
    for row in rows.iter().skip(1) {
        ui.press("j");
        assert_eq!(
            ui.settings_inventory().selected_inventory.as_ref(),
            Some(row)
        );
    }
    // The next Location follows the last Area directly: nothing, such as an add row, sits
    // between.
    assert_eq!(rows[rows.len() - 2], STORAGE);
    assert_eq!(rows.last().map(String::as_str), Some(HILUX));
    ui.press("j");
    assert_eq!(
        ui.settings_inventory().selected_inventory.as_deref(),
        Some(HILUX)
    );
    ui.press("g g");
    assert_eq!(
        ui.settings_inventory().selected_inventory.as_deref(),
        Some(ELM)
    );
    ui.press("shift-g");
    assert_eq!(
        ui.settings_inventory().selected_inventory.as_deref(),
        Some(HILUX)
    );
    ui.press("k k");
    assert!(
        ui.settings_inventory()
            .selected_inventory
            .is_some_and(|row| row.starts_with("area:"))
    );
}

#[gpui::test]
fn left_climbs_from_an_area_and_then_closes_the_location(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    ui.press("right right");
    assert!(
        ui.settings_inventory()
            .selected_inventory
            .is_some_and(|row| row.starts_with("area:"))
    );
    ui.press("left");
    let inventory_page = ui.settings_inventory();
    assert_eq!(inventory_page.selected_inventory.as_deref(), Some(ELM));
    assert!(
        inventory_page.inventory_rows.len() > 2,
        "still open after climbing"
    );
    ui.press("left");
    let inventory_page = ui.settings_inventory();
    let page = ui.settings();
    assert_eq!(inventory_page.inventory_rows, [ELM, STORAGE, HILUX]);
    assert!(page.page_focused, "closing a Location keeps focus");
    // A closed Location has nothing to fold, so left hands focus back to the index.
    ui.press("left");
    assert!(!ui.settings().page_focused);
}

#[gpui::test]
fn shift_j_and_shift_k_reorder_areas_within_their_location(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    ui.press("right right");
    let before = ui.settings_inventory().inventory_areas[0].1.clone();
    let moved = before[0].clone();
    ui.press("shift-j");
    let after = ui.settings_inventory().inventory_areas[0].1.clone();
    assert_eq!(after[1], moved);
    assert_eq!(after[0], before[1]);
    ui.press("shift-k");
    assert_eq!(ui.settings_inventory().inventory_areas[0].1, before);
    // At the top edge it is a no-op.
    ui.press("shift-k");
    assert_eq!(ui.settings_inventory().inventory_areas[0].1, before);
    assert_eq!(
        ui.settings_inventory().selected_inventory,
        Some(format!("area:{moved}"))
    );
}

#[gpui::test]
fn shift_j_is_inert_on_a_location_row_and_at_the_last_area(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);
    let areas = ui.settings_inventory().inventory_areas;

    ui.press("shift-j");
    assert_eq!(ui.settings_inventory().inventory_areas, areas);
    assert_eq!(
        ui.settings_inventory().selected_inventory.as_deref(),
        Some(ELM)
    );

    ui.press("right right");
    for _ in 0..areas[0].1.len() {
        ui.press("j");
    }
    ui.press("shift-j");
    assert_eq!(ui.settings_inventory().inventory_areas, areas);
}

#[gpui::test]
fn n_e_and_x_open_the_location_dialogs_and_escape_cancels(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    ui.press("n");
    assert_eq!(
        ui.settings_inventory().inventory_dialog.as_deref(),
        Some("add")
    );
    ui.press("escape");
    assert_eq!(ui.settings_inventory().inventory_dialog, None);
    assert!(ui.settings().page_focused, "focus returns to the page");
    ui.press("e");
    assert_eq!(
        ui.settings_inventory().inventory_dialog.as_deref(),
        Some("edit")
    );
    ui.press("escape x");
    assert_eq!(
        ui.settings_inventory().inventory_dialog.as_deref(),
        Some("remove")
    );
    ui.press("escape");
    assert_eq!(ui.settings_inventory().inventory_dialog, None);
    ui.press("right right e");
    assert_eq!(
        ui.settings_inventory().inventory_dialog.as_deref(),
        Some("edit-area")
    );
}

fn names(ui: &mut Harness<'_>) -> Vec<String> {
    ui.settings_inventory()
        .inventory_locations
        .into_iter()
        .map(|(name, ..)| name)
        .collect()
}

#[gpui::test]
fn add_needs_a_name_then_adds_expands_selects_and_toasts(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    ui.press("n enter");
    assert_eq!(
        ui.settings_inventory().inventory_dialog.as_deref(),
        Some("add"),
        "enter is inert with no name"
    );
    ui.press("c a b i n enter");
    let inventory_page = ui.settings_inventory();
    assert_eq!(inventory_page.inventory_dialog, None);
    let added = inventory_page.inventory_locations.last().cloned().unwrap();
    assert_eq!(added, ("cabin".to_string(), "aud".to_string(), None));
    assert_eq!(
        inventory_page.selected_inventory.as_deref(),
        Some("location:cabin")
    );
    assert!(
        inventory_page
            .inventory_rows
            .contains(&"location:cabin".to_string())
    );
    assert_eq!(
        ui.toasts().visible.first().map(|t| t.text.clone()),
        Some("Added location \"cabin\"".to_string())
    );
}

#[gpui::test]
fn add_refuses_a_name_another_location_has_ignoring_case(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    ui.press("n s t o r a g e space u n i t enter");
    assert_eq!(
        ui.settings_inventory().inventory_dialog.as_deref(),
        Some("add")
    );
    assert_eq!(names(&mut ui).len(), 3);
}

#[gpui::test]
fn edit_saves_the_name_and_keeps_the_unit_and_cover(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);
    let before = ui.settings_inventory().inventory_locations[0].clone();

    ui.press("e x enter");
    let after = ui.settings_inventory().inventory_locations[0].clone();
    assert_eq!(after.0, format!("{}x", before.0));
    assert_eq!(after.1, before.1);
    assert_eq!(after.2, before.2, "the cover Settings doesn't show is kept");
    assert_eq!(ui.settings_inventory().inventory_dialog, None);
    assert_eq!(
        ui.toasts().visible.first().map(|t| t.text.clone()),
        Some(format!("Saved location \"{}x\"", before.0))
    );
}

#[gpui::test]
fn removing_an_empty_location_is_a_plain_confirm(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    ui.press("n c a b i n enter x enter");
    assert_eq!(ui.settings_inventory().inventory_dialog, None);
    assert_eq!(names(&mut ui).len(), 3);
    assert!(
        ui.toasts()
            .visible
            .iter()
            .any(|t| t.text == "Removed location \"cabin\"")
    );
    assert!(ui.settings_inventory().selected_inventory.is_some());
}

#[gpui::test]
fn removing_a_location_with_contents_asks_for_its_name_typed(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    ui.press("x enter");
    assert_eq!(
        ui.settings_inventory().inventory_dialog.as_deref(),
        Some("remove"),
        "enter is inert until the name is typed"
    );
    ui.press("1 2 space shift-e l m space shift-s t space c o n t e n t s enter");
    let inventory_page = ui.settings_inventory();
    assert_eq!(inventory_page.inventory_dialog, None);
    assert_eq!(
        inventory_page
            .inventory_locations
            .iter()
            .map(|p| p.0.as_str())
            .collect::<Vec<_>>(),
        ["Storage unit", "Hilux"]
    );
    assert_eq!(
        inventory_page.selected_inventory.as_deref(),
        Some("location:Storage unit")
    );
}

#[gpui::test]
fn the_last_location_can_be_removed(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.empty_inventory();
    ui.press("g s");
    ui.press("j j j j j j j j j l n o n e enter");
    assert_eq!(names(&mut ui), vec!["one".to_string()]);
    ui.press("x enter");
    assert!(names(&mut ui).is_empty());
    assert_eq!(ui.settings_inventory().selected_inventory, None);
}

#[gpui::test]
fn a_expands_a_collapsed_location_and_works_from_an_area(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    ui.press("a");
    let inventory_page = ui.settings_inventory();
    assert!(
        inventory_page.inventory_rows.len() > 2,
        "a opens the Location"
    );
    assert_eq!(inventory_page.inventory_dialog.as_deref(), Some("add-area"));
    ui.press("escape right right");
    ui.press("a");
    assert_eq!(
        ui.settings_inventory()
            .selected_inventory
            .as_deref()
            .map(|r| r.starts_with("area:")),
        Some(true)
    );
    assert_eq!(
        ui.settings_inventory().inventory_dialog.as_deref(),
        Some("add-area")
    );
}

#[gpui::test]
fn the_empty_state_ignores_a_e_and_x_but_a_raises_a_toast(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.empty_inventory();
    ui.press("g s");
    ui.press("j j j j j j j j j l");
    let inventory_page = ui.settings_inventory();
    let page = ui.settings();
    assert_eq!(page.page, "Inventory");
    assert!(inventory_page.inventory_rows.is_empty());
    assert_eq!(inventory_page.selected_inventory, None);

    ui.press("e x");
    assert_eq!(ui.settings_inventory().inventory_dialog, None);
    ui.press("j k shift-j right");
    assert!(ui.settings_inventory().inventory_rows.is_empty());
    ui.press("a");
    let toasts = ui.toasts();
    assert_eq!(
        toasts.visible.first().map(|toast| toast.text.as_str()),
        Some("Add a location first")
    );
    ui.press("n");
    assert_eq!(
        ui.settings_inventory().inventory_dialog.as_deref(),
        Some("add")
    );
}

#[gpui::test]
fn the_page_has_no_effect_on_keys_while_the_index_has_focus(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    ui.press("escape");
    assert!(!ui.settings().page_focused);
    ui.press("n a e x");
    assert_eq!(ui.settings_inventory().inventory_dialog, None);
    assert_eq!(ui.settings().page, "Inventory");
}

/// 12 Elm St opened, its first Area selected.
fn on_first_area(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = in_inventory(app);
    ui.press("right right");
    assert!(
        ui.settings_inventory()
            .selected_inventory
            .is_some_and(|r| r.starts_with("area:"))
    );
    ui
}

fn area_names(ui: &mut Harness<'_>, location: usize) -> Vec<String> {
    ui.settings_inventory().inventory_areas[location].1.clone()
}

fn area_items(ui: &mut Harness<'_>, name: &str) -> usize {
    ui.settings_inventory()
        .inventory_area_items
        .iter()
        .find(|(area, _)| area == name)
        .map_or(0, |(_, items)| *items)
}

#[gpui::test]
fn add_area_appends_it_last_selects_it_and_updates_the_count(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);
    let before = area_names(&mut ui, 0).len();

    ui.press("a");
    assert_eq!(
        ui.settings_inventory().inventory_dialog.as_deref(),
        Some("add-area")
    );
    ui.press("enter");
    assert_eq!(
        ui.settings_inventory().inventory_dialog.as_deref(),
        Some("add-area"),
        "an empty name keeps the dialog open"
    );
    ui.press("a t t i c enter");
    let inventory_page = ui.settings_inventory();
    let page = ui.settings();
    assert_eq!(inventory_page.inventory_dialog, None);
    assert!(page.page_focused, "focus returns to the page");
    assert_eq!(area_names(&mut ui, 0).len(), before + 1);
    assert_eq!(
        area_names(&mut ui, 0).last().map(String::as_str),
        Some("attic")
    );
    assert_eq!(
        ui.settings_inventory().selected_inventory.as_deref(),
        Some("area:attic")
    );
    assert_eq!(area_items(&mut ui, "attic"), 0);
    let toasts = ui.toasts();
    assert_eq!(
        toasts.visible.first().map(|toast| toast.text.as_str()),
        Some("Added area \"attic\"")
    );
}

#[gpui::test]
fn an_area_name_may_repeat_in_another_location_but_not_in_its_own(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);
    let living = area_names(&mut ui, 0)[0].clone();

    ui.press("a");
    for ch in living.to_uppercase().chars() {
        ui.press(&ch.to_string());
    }
    ui.press("enter");
    assert_eq!(
        ui.settings_inventory().inventory_dialog.as_deref(),
        Some("add-area"),
        "the same name, ignoring case, is refused in its own Location"
    );
    ui.press("escape");

    // The Storage unit is another Location, so the name is free there.
    ui.press("left j a");
    for ch in living.chars() {
        ui.press(&ch.to_string());
    }
    ui.press("enter");
    assert_eq!(ui.settings_inventory().inventory_dialog, None);
    assert!(area_names(&mut ui, 1).contains(&living));
}

#[gpui::test]
fn edit_area_renames_it_and_keeps_its_items_and_position(app: &mut TestAppContext) {
    let mut ui = on_first_area(app);
    let before = area_names(&mut ui, 0);
    let items = area_items(&mut ui, &before[0]);

    ui.press("e");
    assert_eq!(
        ui.settings_inventory().inventory_dialog.as_deref(),
        Some("edit-area")
    );
    ui.press("escape");
    assert_eq!(ui.settings_inventory().inventory_dialog, None);
    ui.press("e backspace x enter");
    let after = area_names(&mut ui, 0);
    assert_eq!(after.len(), before.len());
    assert_eq!(after[0], format!("{}x", &before[0][..before[0].len() - 1]));
    assert_eq!(area_items(&mut ui, &after[0]), items);
    assert_eq!(&after[1..], &before[1..]);
}

#[gpui::test]
fn removing_an_empty_area_is_a_plain_confirm(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);
    ui.press("a a t t i c enter");
    let with_attic = area_names(&mut ui, 0);

    ui.press("x");
    assert_eq!(
        ui.settings_inventory().inventory_dialog.as_deref(),
        Some("remove-area")
    );
    ui.press("escape");
    assert_eq!(area_names(&mut ui, 0), with_attic);
    ui.press("x enter");
    let inventory_page = ui.settings_inventory();
    assert_eq!(inventory_page.inventory_dialog, None);
    assert_eq!(area_names(&mut ui, 0).len(), with_attic.len() - 1);
    let toasts = ui.toasts();
    assert!(
        toasts
            .visible
            .iter()
            .any(|toast| toast.text == "Removed area \"attic\"")
    );
    assert!(
        ui.settings_inventory()
            .selected_inventory
            .is_some_and(|r| r.starts_with("area:"))
    );
}

#[gpui::test]
fn removing_an_area_with_items_moves_them_to_the_area_above_by_default(app: &mut TestAppContext) {
    let mut ui = on_first_area(app);
    ui.press("j");
    let areas = area_names(&mut ui, 0);
    let (second, first) = (areas[1].clone(), areas[0].clone());
    let moving = area_items(&mut ui, &second);
    let first_before = area_items(&mut ui, &first);
    let total = ui
        .settings_inventory()
        .inventory_area_items
        .iter()
        .map(|r| r.1)
        .sum::<usize>();

    ui.press("x");
    assert_eq!(
        ui.settings_inventory().inventory_dialog.as_deref(),
        Some("remove-area")
    );
    ui.press("enter");
    assert_eq!(ui.settings_inventory().inventory_dialog, None);
    assert!(!area_names(&mut ui, 0).contains(&second));
    assert_eq!(area_items(&mut ui, &first), first_before + moving);
    assert_eq!(
        ui.settings_inventory()
            .inventory_area_items
            .iter()
            .map(|r| r.1)
            .sum::<usize>(),
        total,
        "no Item is lost"
    );
}

#[gpui::test]
fn the_first_area_pre_selects_the_area_below_and_the_picker_can_change_it(
    app: &mut TestAppContext,
) {
    let mut ui = on_first_area(app);
    let areas = area_names(&mut ui, 0);
    let moving = area_items(&mut ui, &areas[0]);
    let second_before = area_items(&mut ui, &areas[1]);
    let third_before = area_items(&mut ui, &areas[2]);

    ui.press("x space down enter");
    assert_eq!(
        ui.settings_inventory().inventory_dialog.as_deref(),
        Some("remove-area"),
        "enter on the open list picks, it does not confirm"
    );
    ui.press("enter");
    assert_eq!(area_items(&mut ui, &areas[1]), second_before);
    assert_eq!(area_items(&mut ui, &areas[2]), third_before + moving);
}

#[gpui::test]
fn the_last_area_with_items_is_blocked_and_nothing_changes(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);
    // The Storage unit has one Area, holding Items.
    ui.press("j right right x");
    assert_eq!(
        ui.settings_inventory().inventory_dialog.as_deref(),
        Some("area-blocked")
    );
    ui.press("enter");
    assert_eq!(ui.settings_inventory().inventory_dialog, None);
    assert_eq!(area_names(&mut ui, 1).len(), 1);
    ui.press("x escape");
    assert_eq!(ui.settings_inventory().inventory_dialog, None);
    assert!(ui.settings().page_focused);
}

#[gpui::test]
fn the_status_line_reads_the_live_counts_and_a_adds_an_area(app: &mut TestAppContext) {
    let mut ui = in_inventory(app);

    let status = ui.status().expect("the Inventory page has a legend");
    assert_eq!(status.right, "3 locations \u{b7} 14 areas \u{b7} 241 items");
    let keys: Vec<&str> = status.hints.iter().map(|(key, _)| key.as_str()).collect();
    assert!(keys.contains(&"a"), "a adds an area: {keys:?}");
    assert!(!keys.contains(&"r"), "r no longer adds an area: {keys:?}");
}
