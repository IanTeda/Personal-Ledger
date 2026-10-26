//! The Settings Categories page driven by keys: row movement, the add, edit and delete dialogs,
//! and Esc closing each without a write. State is read back from `Root`.

use super::common;

use bin_desktop::navigation::nav::InputMode;
use common::Harness;
use gpui::TestAppContext;

fn mode(ui: &mut Harness<'_>) -> InputMode {
    ui.read(|shell| shell.nav().mode())
}

/// Settings on the Categories page (five presses down the index) with focus stepped into it.
fn in_categories(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    ui.press("j j j j j");
    assert_eq!(ui.settings().page, "Categories");
    ui.press("l");
    assert!(ui.settings().page_focused);
    ui
}

#[gpui::test]
fn rows_move_with_j_k_and_the_ends(app: &mut TestAppContext) {
    let mut ui = in_categories(app);
    assert_eq!(ui.settings_categories().selected_category, None);

    ui.press("j");
    let first = ui.settings_categories().selected_category;
    assert!(first.is_some());
    ui.press("j");
    let second = ui.settings_categories().selected_category;
    assert_ne!(first, second);
    ui.press("k");
    assert_eq!(ui.settings_categories().selected_category, first);
    ui.press("shift-g");
    // Interest is the last row: the Income tree sits below the Expense one.
    assert_eq!(
        ui.settings_categories().selected_category.as_deref(),
        Some("Interest")
    );
    ui.press("g g");
    assert_eq!(ui.settings_categories().selected_category, first);
}

#[gpui::test]
fn n_opens_add_category_and_escape_cancels_it(app: &mut TestAppContext) {
    let mut ui = in_categories(app);
    let before = ui.settings_categories().category_names;

    ui.press("n");
    assert_eq!(ui.dialog(), Some("AddCategory"));
    assert_eq!(mode(&mut ui), InputMode::Dialog);
    ui.press("p e t s");
    ui.press("escape");
    assert_eq!(ui.dialog(), None);
    assert_eq!(mode(&mut ui), InputMode::Normal);
    assert_eq!(ui.settings_categories().category_names, before);
}

#[gpui::test]
fn typing_a_name_and_pressing_enter_adds_the_category(app: &mut TestAppContext) {
    let mut ui = in_categories(app);
    let before = ui.settings_categories().category_names.len();

    ui.press("n");
    ui.press("p e t s");
    assert_eq!(ui.dialog_value("name").as_deref(), Some("pets"));
    ui.press("enter");
    let categories_page = ui.settings_categories();
    assert_eq!(ui.dialog(), None);
    assert_eq!(categories_page.category_names.len(), before + 1);
    assert!(
        categories_page
            .category_names
            .iter()
            .any(|name| name == "pets")
    );
}

#[gpui::test]
fn a_failed_write_puts_the_categories_back_with_an_error_toast(app: &mut TestAppContext) {
    let mut ui = Harness::with_failing_categories(app);
    ui.press("g s");
    ui.press("j j j j j");
    ui.press("l");
    let stored = ui.settings_categories().category_names;

    ui.press("n p e t s enter");
    assert_eq!(
        ui.settings_categories().category_names.len(),
        stored.len() + 1,
        "memory runs ahead of the file until the failure is heard"
    );
    ui.settle_writes();

    assert_eq!(ui.settings_categories().category_names, stored);
    assert!(ui.toasts().visible.into_iter().any(|toast| {
        toast.kind == "Error"
            && toast.text
                == "Couldn't save the change to categories, so they are back as last saved"
    }));
}

#[gpui::test]
fn shift_n_opens_add_under_the_selected_row(app: &mut TestAppContext) {
    let mut ui = in_categories(app);

    // Housing is the first row and a top-level Category, so it can take a sub-category.
    ui.press("j shift-n");
    assert_eq!(ui.dialog(), Some("AddCategory"));
    assert_eq!(ui.dialog_value("parent").as_deref(), Some("Housing"));
    ui.press("escape");
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn n_opens_a_top_level_add_with_no_parent(app: &mut TestAppContext) {
    let mut ui = in_categories(app);

    ui.press("j n");
    assert_eq!(ui.dialog(), Some("AddCategory"));
    assert_eq!(ui.dialog_value("parent"), None);
    ui.press("escape");
}

#[gpui::test]
fn enter_on_a_blank_name_keeps_the_add_dialog_open(app: &mut TestAppContext) {
    let mut ui = in_categories(app);
    let before = ui.settings_categories().category_names.len();

    ui.press("n enter");
    assert_eq!(ui.dialog(), Some("AddCategory"));
    assert_eq!(ui.settings_categories().category_names.len(), before);
}

#[gpui::test]
fn e_edits_the_selected_category_and_enter_saves_the_new_name(app: &mut TestAppContext) {
    let mut ui = in_categories(app);

    // The first row is Housing, a parent; renaming it leaves its children where they are.
    ui.press("j e");
    assert_eq!(ui.dialog(), Some("EditCategory"));
    let selected = ui.settings_categories().selected_category;
    assert_eq!(ui.dialog_value("name"), selected);
    ui.press("x");
    ui.press("enter");
    let categories_page = ui.settings_categories();
    assert_eq!(ui.dialog(), None);
    assert_eq!(
        categories_page.selected_category,
        selected.map(|name| format!("{name}x"))
    );
    assert!(
        categories_page
            .category_names
            .iter()
            .any(|name| name.ends_with('x'))
    );
}

#[gpui::test]
fn escape_closes_the_edit_dialog_without_renaming(app: &mut TestAppContext) {
    let mut ui = in_categories(app);
    let before = ui.settings_categories().category_names;

    ui.press("j e x");
    assert_eq!(ui.dialog(), Some("EditCategory"));
    ui.press("escape");
    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.settings_categories().category_names, before);
}

#[gpui::test]
fn d_on_a_parent_is_refused_and_a_leaf_opens_the_delete_dialog(app: &mut TestAppContext) {
    let mut ui = in_categories(app);

    ui.press("j d");
    let page = ui.settings();
    assert_eq!(ui.dialog(), None);
    assert_eq!(
        page.status_message.as_deref(),
        Some("delete or move its children first")
    );
    // Right opens Housing and lands on its first child, Rent, a leaf.
    ui.press("right");
    assert_eq!(
        ui.settings_categories().selected_category.as_deref(),
        Some("Rent")
    );
    ui.press("d");
    assert_eq!(ui.dialog(), Some("DeleteCategory"));
    assert_eq!(mode(&mut ui), InputMode::Dialog);
}

#[gpui::test]
fn escape_closes_the_delete_dialog_without_removing(app: &mut TestAppContext) {
    let mut ui = in_categories(app);
    let before = ui.settings_categories().category_names;

    ui.press("j right d");
    assert_eq!(ui.dialog(), Some("DeleteCategory"));
    ui.press("escape");
    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.settings_categories().category_names, before);
}

#[gpui::test]
fn typing_the_exact_name_and_pressing_enter_removes_the_leaf(app: &mut TestAppContext) {
    let mut ui = in_categories(app);

    // Rent is used by splits, so they move to Uncategorised: the count can stay level, so the
    // check is on the name.
    ui.press("j right d");
    // The delete confirm is gated on the exact, case-sensitive name, so enter does nothing yet.
    ui.press("enter");
    assert_eq!(ui.dialog(), Some("DeleteCategory"));
    ui.press("shift-r e n t");
    assert_eq!(ui.dialog_value("confirm").as_deref(), Some("Rent"));
    ui.press("enter");
    let categories_page = ui.settings_categories();
    assert_eq!(ui.dialog(), None);
    assert!(
        !categories_page
            .category_names
            .iter()
            .any(|name| name == "Rent")
    );
}

#[gpui::test]
fn left_climbs_from_a_nested_row_to_its_parent_and_keeps_the_page_focused(
    app: &mut TestAppContext,
) {
    let mut ui = in_categories(app);
    // Rent is the first nested row, under the open Housing parent.
    for _ in 0..10 {
        if ui.settings_categories().selected_category.as_deref() == Some("Rent") {
            break;
        }
        ui.press("j");
    }
    assert_eq!(
        ui.settings_categories().selected_category.as_deref(),
        Some("Rent")
    );

    ui.press("left");
    assert_eq!(
        ui.settings_categories().selected_category.as_deref(),
        Some("Housing")
    );
    assert!(ui.settings().page_focused);
}

#[gpui::test]
fn the_edit_type_change_is_held_until_enter_and_dropped_by_escape(app: &mut TestAppContext) {
    let mut ui = in_categories(app);
    let before = ui.settings_categories().category_types;

    // The first row is Housing, a top-level Expense with children.
    ui.press("j");
    ui.press("e");
    assert_eq!(ui.dialog(), Some("EditCategory"));
    ui.click("edit-type-income");
    assert_eq!(ui.settings_categories().category_types, before);
    ui.press("escape");
    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.settings_categories().category_types, before);

    ui.press("e");
    ui.click("edit-type-income");
    ui.press("enter");
    assert_eq!(ui.dialog(), None);
    assert_ne!(ui.settings_categories().category_types, before);
}

#[gpui::test]
fn the_edit_dialog_hides_parents_that_would_nest_the_subtree_past_three_levels(
    app: &mut TestAppContext,
) {
    let mut ui = in_categories(app);

    // Housing (height 2) can't go under Transport; its Utilities (height 1) can.
    ui.press("j e");
    assert_eq!(ui.dialog_value("name").as_deref(), Some("Housing"));
    let parents = ui.dialog_value("parents").expect("the offered parents");
    assert!(!parents.split(',').any(|name| name == "Transport"));
    ui.press("escape");

    ui.press("j j e");
    assert_eq!(ui.dialog_value("name").as_deref(), Some("Utilities"));
    let parents = ui.dialog_value("parents").expect("the offered parents");
    assert!(parents.split(',').any(|name| name == "Transport"));
}
