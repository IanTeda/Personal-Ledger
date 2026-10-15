//! The Settings Accounts page driven by clicks: the add button, a row opening its ledger, and the
//! rows' edit and delete actions. The dialogs' own forms are in `accounts_mouse.rs`. Only elements
//! tagged with `debug_selector` are clicked; state is read back from `Shell`.

use super::common;

use bin_desktop::navigation::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;

fn mode(ui: &mut Harness<'_>) -> InputMode {
    ui.read(|shell| shell.nav().mode())
}

/// Settings with the Accounts page on show, focus left on the index.
fn on_accounts(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    ui.click("settings-index-Accounts");
    assert_eq!(ui.settings().page, "Accounts");
    ui
}

#[gpui::test]
fn the_add_button_opens_the_dialog_and_cancel_closes_it(app: &mut TestAppContext) {
    let mut ui = on_accounts(app);

    ui.click("settings-accounts-add");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddAccount"));
    assert_eq!(mode(&mut ui), InputMode::Dialog);
    ui.click("add-account-cancel");
    assert_eq!(ui.settings().dialog, None);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn clicking_an_account_row_selects_it(app: &mut TestAppContext) {
    let mut ui = on_accounts(app);

    let first = ui.settings_accounts().selected_account;
    ui.click("settings-accounts-row-2");
    let second = ui.settings_accounts().selected_account;
    assert!(second.is_some());
    assert_ne!(first, second);
}

#[gpui::test]
fn an_edit_action_opens_its_dialog_with_the_name(app: &mut TestAppContext) {
    let mut ui = on_accounts(app);

    ui.click("settings-accounts-edit-2");
    let accounts_page = ui.settings_accounts();
    let page = ui.settings();
    assert_eq!(page.dialog.as_deref(), Some("EditAccount"));
    assert_eq!(page.dialog_name, accounts_page.selected_account);
    ui.click("edit-account-cancel");
    assert_eq!(ui.settings().dialog, None);
}

#[gpui::test]
fn a_delete_action_needs_the_name_typed_back(app: &mut TestAppContext) {
    let mut ui = on_accounts(app);
    let before = ui.settings_accounts().account_names.len();

    ui.click("settings-accounts-delete-2");
    assert_eq!(ui.settings().dialog.as_deref(), Some("DeleteAccount"));
    ui.click("delete-account-confirm");
    assert_eq!(ui.settings().dialog.as_deref(), Some("DeleteAccount"));
    assert_eq!(ui.settings_accounts().account_names.len(), before);
    ui.click("delete-account-cancel");
    assert_eq!(ui.settings().dialog, None);
}

#[gpui::test]
fn clicking_an_account_row_opens_its_ledger(app: &mut TestAppContext) {
    let mut ui = on_accounts(app);

    ui.click("settings-accounts-row-2");
    let name = ui
        .settings_accounts()
        .selected_account
        .expect("the clicked row is selected");
    assert_eq!(ui.noun(), Noun::Transactions);
    assert!(
        ui.transactions()
            .chips
            .iter()
            .any(|chip| chip.active && chip.label == format!("account: {name}"))
    );
}

#[gpui::test]
fn an_edit_action_saved_keeps_the_edited_account_selected(app: &mut TestAppContext) {
    let mut ui = on_accounts(app);

    ui.click("settings-accounts-edit-2");
    let name = ui.settings().dialog_name.expect("the Edit dialog is open");
    ui.press("x");
    ui.click("edit-account-confirm");
    assert_eq!(ui.settings().dialog, None);
    assert_eq!(mode(&mut ui), InputMode::Normal);
    assert_eq!(
        ui.settings_accounts().selected_account,
        Some(format!("{name}x"))
    );
}
