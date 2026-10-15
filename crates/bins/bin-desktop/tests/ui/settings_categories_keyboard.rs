//! The Settings Categories page driven by keys: row movement, the add, edit and delete dialogs,
//! and Esc closing each without a write. State is read back from `Shell`.

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
    assert_ne!(ui.settings_categories().selected_category, first);
    ui.press("g g");
    assert_eq!(ui.settings_categories().selected_category, first);
}

#[gpui::test]
fn n_opens_add_category_and_escape_cancels_it(app: &mut TestAppContext) {
    let mut ui = in_categories(app);
    let before = ui.settings_categories().category_names;

    ui.press("n");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddCategory"));
    assert_eq!(mode(&mut ui), InputMode::Dialog);
    ui.press("p e t s");
    ui.press("escape");
    assert_eq!(ui.settings().dialog, None);
    assert_eq!(mode(&mut ui), InputMode::Normal);
    assert_eq!(ui.settings_categories().category_names, before);
}

#[gpui::test]
fn typing_a_name_and_pressing_enter_adds_the_category(app: &mut TestAppContext) {
    let mut ui = in_categories(app);
    let before = ui.settings_categories().category_names.len();

    ui.press("n");
    ui.press("p e t s");
    assert_eq!(ui.settings().dialog_name.as_deref(), Some("pets"));
    ui.press("enter");
    let categories_page = ui.settings_categories();
    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert_eq!(categories_page.category_names.len(), before + 1);
    assert!(
        categories_page
            .category_names
            .iter()
            .any(|name| name == "pets")
    );
}

#[gpui::test]
fn enter_on_a_blank_name_keeps_the_add_dialog_open(app: &mut TestAppContext) {
    let mut ui = in_categories(app);
    let before = ui.settings_categories().category_names.len();

    ui.press("n enter");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddCategory"));
    assert_eq!(ui.settings_categories().category_names.len(), before);
}

#[gpui::test]
fn e_edits_the_selected_category_and_enter_saves_the_new_name(app: &mut TestAppContext) {
    let mut ui = in_categories(app);

    // The first row is Housing, a parent; renaming it leaves its children where they are.
    ui.press("j e");
    let page = ui.settings();
    assert_eq!(page.dialog.as_deref(), Some("EditCategory"));
    let selected = ui.settings_categories().selected_category;
    assert_eq!(page.dialog_name, selected);
    ui.press("x");
    ui.press("enter");
    let categories_page = ui.settings_categories();
    let page = ui.settings();
    assert_eq!(page.dialog, None);
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
    assert_eq!(ui.settings().dialog.as_deref(), Some("EditCategory"));
    ui.press("escape");
    assert_eq!(ui.settings().dialog, None);
    assert_eq!(ui.settings_categories().category_names, before);
}

#[gpui::test]
fn d_on_a_parent_is_refused_and_a_leaf_opens_the_delete_dialog(app: &mut TestAppContext) {
    let mut ui = in_categories(app);

    ui.press("j d");
    assert_eq!(ui.settings().dialog, None);
    // Right opens Housing and lands on its first child, Rent, a leaf.
    ui.press("right");
    assert_eq!(
        ui.settings_categories().selected_category.as_deref(),
        Some("Rent")
    );
    ui.press("d");
    assert_eq!(ui.settings().dialog.as_deref(), Some("DeleteCategory"));
    assert_eq!(mode(&mut ui), InputMode::Dialog);
}

#[gpui::test]
fn escape_closes_the_delete_dialog_without_removing(app: &mut TestAppContext) {
    let mut ui = in_categories(app);
    let before = ui.settings_categories().category_names;

    ui.press("j right d");
    assert_eq!(ui.settings().dialog.as_deref(), Some("DeleteCategory"));
    ui.press("escape");
    assert_eq!(ui.settings().dialog, None);
    assert_eq!(ui.settings_categories().category_names, before);
}

#[gpui::test]
fn typing_the_exact_name_and_pressing_enter_removes_the_leaf(app: &mut TestAppContext) {
    let mut ui = in_categories(app);

    // Rent has splits, so its rows move to Uncategorised: the count can stay level, so the
    // check is on the name.
    ui.press("j right d");
    // The delete confirm is gated on the exact, case-sensitive name, so enter does nothing yet.
    ui.press("enter");
    assert_eq!(ui.settings().dialog.as_deref(), Some("DeleteCategory"));
    ui.press("shift-r e n t");
    assert_eq!(ui.settings().dialog_confirm.as_deref(), Some("Rent"));
    ui.press("enter");
    let categories_page = ui.settings_categories();
    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert!(
        !categories_page
            .category_names
            .iter()
            .any(|name| name == "Rent")
    );
}
