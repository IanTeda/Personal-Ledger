//! The Settings pages driven by clicks: the index rail, the Accounts, Categories, Payees and
//! Units pages' add button and row actions, and the dialogs' buttons. Only elements tagged with
//! `debug_selector` are clicked; state is read back from `Shell`.

mod common;

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
fn the_accounts_add_button_opens_the_dialog_and_cancel_closes_it(app: &mut TestAppContext) {
    let mut ui = on_settings(app);
    ui.click("settings-index-Accounts");

    ui.click("settings-accounts-add");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddAccount"));
    assert_eq!(mode(&mut ui), InputMode::Dialog);
    ui.click("add-account-cancel");
    assert_eq!(ui.settings().dialog, None);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn clicking_an_account_row_selects_it(app: &mut TestAppContext) {
    let mut ui = on_settings(app);
    ui.click("settings-index-Accounts");

    let first = ui.settings().selected_account;
    ui.click("settings-accounts-row-2");
    let second = ui.settings().selected_account;
    assert!(second.is_some());
    assert_ne!(first, second);
}

#[gpui::test]
fn an_accounts_edit_action_opens_its_dialog_with_the_name(app: &mut TestAppContext) {
    let mut ui = on_settings(app);
    ui.click("settings-index-Accounts");

    ui.click("settings-accounts-edit-2");
    let page = ui.settings();
    assert_eq!(page.dialog.as_deref(), Some("EditAccount"));
    assert_eq!(page.dialog_name, page.selected_account);
    ui.click("edit-account-cancel");
    assert_eq!(ui.settings().dialog, None);
}

#[gpui::test]
fn an_accounts_delete_action_needs_the_name_typed_back(app: &mut TestAppContext) {
    let mut ui = on_settings(app);
    ui.click("settings-index-Accounts");
    let before = ui.settings().account_names.len();

    ui.click("settings-accounts-delete-2");
    assert_eq!(ui.settings().dialog.as_deref(), Some("DeleteAccount"));
    ui.click("delete-account-confirm");
    assert_eq!(ui.settings().dialog.as_deref(), Some("DeleteAccount"));
    assert_eq!(ui.settings().account_names.len(), before);
    ui.click("delete-account-cancel");
    assert_eq!(ui.settings().dialog, None);
}

#[gpui::test]
fn the_payees_add_button_and_row_actions_open_their_dialogs(app: &mut TestAppContext) {
    let mut ui = on_settings(app);
    ui.click("settings-index-Payees");

    ui.click("settings-payees-add");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddPayee"));
    ui.click("add-payee-cancel");
    assert_eq!(ui.settings().dialog, None);

    ui.click("settings-payees-edit-1");
    assert_eq!(ui.settings().dialog.as_deref(), Some("EditPayee"));
    ui.press("escape");
    ui.click("settings-payees-delete-1");
    assert_eq!(ui.settings().dialog.as_deref(), Some("DeletePayee"));
    ui.click("delete-payee-cancel");
    assert_eq!(ui.settings().dialog, None);
}

#[gpui::test]
fn clicking_a_payee_row_selects_it(app: &mut TestAppContext) {
    let mut ui = on_settings(app);
    ui.click("settings-index-Payees");

    let first = ui.settings().selected_payee;
    ui.click("settings-payees-row-2");
    let page = ui.settings();
    assert!(page.selected_payee.is_some());
    assert_ne!(first, page.selected_payee);
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
    let page = ui.settings();
    assert_eq!(page.dialog.as_deref(), Some("EditCategory"));
    assert_eq!(page.dialog_name, page.selected_category);
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

    let first = ui.settings().unit_codes[0].clone();
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
    let before = ui.settings().unit_codes.clone();

    ui.click("unit-delete-0");
    for ch in before[0].chars() {
        ui.press(&ch.to_string());
    }
    ui.click("delete-unit-confirm");
    let after = ui.settings().unit_codes;
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
