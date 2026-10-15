//! The Settings pages driven by clicks: the index rail, the Categories and Units pages' add
//! button and row actions, and the dialogs' buttons. Only elements tagged with
//! `debug_selector` are clicked; state is read back from `Shell`.

use super::common;

use bin_desktop::navigation::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;

fn on_settings(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    ui
}

fn mode(ui: &mut Harness<'_>) -> InputMode {
    ui.read(|shell| shell.nav().mode())
}

#[gpui::test]
fn clicking_an_index_entry_swaps_the_page(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    ui.click("settings-index-Accounts");
    assert_eq!(ui.settings().page, "Accounts");
    ui.click("settings-index-About");
    assert_eq!(ui.settings().page, "About");
    ui.click("settings-index-General");
    assert_eq!(ui.settings().page, "General");
}

#[gpui::test]
fn the_categories_add_button_and_row_actions_open_their_dialogs(app: &mut TestAppContext) {
    let mut ui = on_settings(app);
    ui.click("settings-index-Categories");

    ui.click("settings-categories-add");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddCategory"));
    ui.click("add-category-cancel");
    assert_eq!(ui.settings().dialog, None);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn a_categories_edit_action_selects_the_row_and_opens_its_dialog(app: &mut TestAppContext) {
    let mut ui = on_settings(app);
    ui.click("settings-index-Categories");

    ui.click("settings-categories-edit-1");
    let categories_page = ui.settings_categories();
    let page = ui.settings();
    assert_eq!(page.dialog.as_deref(), Some("EditCategory"));
    assert_eq!(page.dialog_name, categories_page.selected_category);
    ui.click("edit-category-cancel");
    assert_eq!(ui.settings().dialog, None);
}

#[gpui::test]
fn the_units_add_edit_and_delete_buttons_open_their_dialogs(app: &mut TestAppContext) {
    let mut ui = on_settings(app);
    ui.click("settings-index-Units");

    ui.click("settings-add-unit");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddUnit"));
    ui.click("add-unit-cancel");
    assert_eq!(ui.settings().dialog, None);

    let first = ui.settings_units().unit_codes[0].clone();
    ui.click("unit-edit-0");
    let page = ui.settings();
    assert_eq!(page.dialog.as_deref(), Some("EditUnit"));
    assert_eq!(page.dialog_name.as_deref(), Some(first.as_str()));
    ui.click("edit-unit-cancel");
    assert_eq!(ui.settings().dialog, None);

    ui.click("unit-delete-0");
    assert_eq!(ui.settings().dialog.as_deref(), Some("DeleteUnit"));
    ui.click("delete-unit-confirm");
    assert_eq!(ui.settings().dialog.as_deref(), Some("DeleteUnit"));
    ui.click("delete-unit-cancel");
    assert_eq!(ui.settings().dialog, None);
}

#[gpui::test]
fn deleting_a_unit_by_mouse_removes_its_row_once_the_code_is_typed(app: &mut TestAppContext) {
    let mut ui = on_settings(app);
    ui.click("settings-index-Units");
    let before = ui.settings_units().unit_codes.clone();

    ui.click("unit-delete-0");
    for ch in before[0].chars() {
        ui.press(&ch.to_string());
    }
    ui.click("delete-unit-confirm");
    let after = ui.settings_units().unit_codes;
    assert_eq!(after.len(), before.len() - 1);
    assert!(!after.contains(&before[0]));
}

#[gpui::test]
fn the_about_page_shows_without_a_focus_target(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    ui.click("settings-index-About");
    let page = ui.settings();
    assert_eq!(page.page, "About");
    assert!(!page.page_focused);
}
