//! The Categories page driven by clicks: the index rail, the add button, the row edit action
//! and the dialog buttons. Only elements tagged with `debug_selector` are clicked; state is read
//! back from the Settings snapshot.

mod common;

use bin_desktop::navigation::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;

fn on_categories(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    ui.click("settings-index-Categories");
    assert_eq!(ui.settings().page, "Categories");
    ui
}

fn mode(ui: &mut Harness<'_>) -> InputMode {
    ui.read(|shell| shell.nav().mode())
}

#[gpui::test]
fn the_add_button_opens_the_dialog_and_cancel_closes_it(app: &mut TestAppContext) {
    let mut ui = on_categories(app);

    ui.click("settings-categories-add");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddCategory"));
    assert_eq!(mode(&mut ui), InputMode::Dialog);
    ui.click("add-category-cancel");
    assert_eq!(ui.settings().dialog, None);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn an_edit_action_selects_the_row_and_opens_its_dialog(app: &mut TestAppContext) {
    let mut ui = on_categories(app);

    ui.click("settings-categories-edit-1");
    let page = ui.settings();
    assert_eq!(page.dialog.as_deref(), Some("EditCategory"));
    assert_eq!(page.dialog_name, page.selected_category);
    ui.click("edit-category-cancel");
    assert_eq!(ui.settings().dialog, None);
}
