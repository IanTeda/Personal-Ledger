//! The Categories page driven by keys: row selection, the edit, add and delete dialogs, and
//! the add-by-typing flow. Categories is a Settings page, so the state is read back from the
//! Settings snapshot.

mod common;

use bin_desktop::navigation::nav::Noun;
use common::Harness;
use gpui::TestAppContext;

fn on_categories(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    ui.press("j j j j j");
    assert_eq!(ui.settings().page, "Categories");
    ui.press("l");
    assert!(ui.settings().page_focused);
    ui
}

#[gpui::test]
fn opens_on_the_categories_page_with_no_row_selected(app: &mut TestAppContext) {
    let mut ui = on_categories(app);

    let page = ui.settings();
    assert!(!page.category_names.is_empty());
    assert_eq!(page.selected_category, None);
    assert_eq!(page.dialog, None);
}

#[gpui::test]
fn j_moves_the_selection_onto_a_category(app: &mut TestAppContext) {
    let mut ui = on_categories(app);

    ui.press("j");
    assert!(ui.settings().selected_category.is_some());
}

#[gpui::test]
fn e_opens_the_edit_dialog_and_escape_closes_it(app: &mut TestAppContext) {
    let mut ui = on_categories(app);

    ui.press("j e");
    assert_eq!(ui.settings().dialog.as_deref(), Some("EditCategory"));
    ui.press("escape");
    assert_eq!(ui.settings().dialog, None);
}

#[gpui::test]
fn shift_n_and_n_open_the_add_dialog(app: &mut TestAppContext) {
    let mut ui = on_categories(app);

    ui.press("j");
    ui.press("shift-n");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddCategory"));
    ui.press("escape");
    ui.press("n");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddCategory"));
    ui.press("escape");
    assert_eq!(ui.settings().dialog, None);
}

#[gpui::test]
fn a_new_category_is_added_by_typing_its_name(app: &mut TestAppContext) {
    let mut ui = on_categories(app);
    let before = ui.settings().category_names.len();

    ui.press("n");
    ui.press("p e t s");
    assert_eq!(ui.settings().dialog_name.as_deref(), Some("pets"));
    ui.press("enter");
    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert_eq!(page.category_names.len(), before + 1);
    assert!(page.category_names.iter().any(|name| name == "pets"));
}

#[gpui::test]
fn a_parent_category_cannot_be_deleted_but_a_leaf_can(app: &mut TestAppContext) {
    let mut ui = on_categories(app);

    ui.press("j");
    ui.press("d");
    assert_eq!(ui.settings().dialog, None);
    ui.press("right");
    assert_eq!(ui.settings().selected_category.as_deref(), Some("Rent"));
    ui.press("d");
    assert_eq!(ui.settings().dialog.as_deref(), Some("DeleteCategory"));
    ui.press("escape");
    assert_eq!(ui.settings().dialog, None);
}
