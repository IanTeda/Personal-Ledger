//! The Settings Accounts page driven by clicks: the add button, a row opening its ledger, and the
//! rows' edit and delete actions. The dialogs' own forms are in `accounts_mouse.rs`. Only elements
//! tagged with `debug_selector` are clicked; state is read back from `Root`.

use super::common;
use lib_core::AccountId;

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
    assert_eq!(ui.dialog(), Some("AddAccount"));
    assert_eq!(mode(&mut ui), InputMode::Dialog);
    ui.click("add-account-cancel");
    assert_eq!(ui.dialog(), None);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn clicking_an_account_row_selects_it(app: &mut TestAppContext) {
    let mut ui = on_accounts(app);

    let first = ui.settings_accounts().selected_account;
    ui.click(&format!("settings-accounts-row-{}", AccountId::seed(2)));
    let second = ui.settings_accounts().selected_account;
    assert!(second.is_some());
    assert_ne!(first, second);
}

#[gpui::test]
fn an_edit_action_opens_its_dialog_with_the_name(app: &mut TestAppContext) {
    let mut ui = on_accounts(app);

    ui.click(&format!("settings-accounts-edit-{}", AccountId::seed(2)));
    let accounts_page = ui.settings_accounts();
    assert_eq!(ui.dialog(), Some("EditAccount"));
    assert_eq!(ui.dialog_value("name"), accounts_page.selected_account);
    ui.click("edit-account-cancel");
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn a_delete_action_needs_the_name_typed_back(app: &mut TestAppContext) {
    let mut ui = on_accounts(app);
    let before = ui.settings_accounts().account_names.len();

    ui.click(&format!("settings-accounts-delete-{}", AccountId::seed(2)));
    assert_eq!(ui.dialog(), Some("DeleteAccount"));
    ui.click("delete-account-confirm");
    assert_eq!(ui.dialog(), Some("DeleteAccount"));
    assert_eq!(ui.settings_accounts().account_names.len(), before);
    ui.click("delete-account-cancel");
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn clicking_an_account_row_opens_its_ledger(app: &mut TestAppContext) {
    let mut ui = on_accounts(app);

    ui.click(&format!("settings-accounts-row-{}", AccountId::seed(2)));
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

    ui.click(&format!("settings-accounts-edit-{}", AccountId::seed(2)));
    let name = ui.dialog_value("name").expect("the Edit dialog is open");
    ui.press("x");
    ui.click("edit-account-confirm");
    assert_eq!(ui.dialog(), None);
    assert_eq!(mode(&mut ui), InputMode::Normal);
    assert_eq!(
        ui.settings_accounts().selected_account,
        Some(format!("{name}x"))
    );
}

#[gpui::test]
fn a_new_account_defaults_to_the_base_unit_set_on_general(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("g s");
    ui.click("settings-general-base-unit-btc");
    ui.press("j j j j");
    assert_eq!(ui.settings().page, "Accounts");
    ui.press("l");

    ui.press("n");
    assert_eq!(ui.dialog(), Some("AddAccount"));
    assert_eq!(ui.account_form().unit.as_deref(), Some("btc"));
}

#[gpui::test]
fn a_deactivated_unit_is_missing_from_the_add_account_unit_picker(app: &mut TestAppContext) {
    let mut ui = on_accounts(app);
    ui.click("settings-index-Units");
    ui.right_click("unit-row-1");
    ui.click("settings-units-menu-delete-unit");
    // A Unit in use opens its Delete dialog as Deactivate.
    ui.click("delete-unit-confirm");

    ui.click("settings-index-Accounts");
    ui.click("settings-accounts-add");
    let offered = ui.dialog_value("unit_options").unwrap_or_default();
    assert!(!offered.split('|').any(|code| code == "btc"), "{offered}");
    assert!(offered.split('|').any(|code| code == "aud"));
}

#[gpui::test]
fn an_account_on_a_deactivated_unit_still_edits_with_its_unit(app: &mut TestAppContext) {
    let mut ui = on_accounts(app);
    ui.click("settings-index-Units");
    ui.right_click("unit-row-1");
    ui.click("settings-units-menu-delete-unit");
    // A Unit in use opens its Delete dialog as Deactivate.
    ui.click("delete-unit-confirm");
    assert_eq!(ui.inactive_unit_codes(), vec!["btc".to_string()]);

    ui.click("settings-index-Accounts");
    ui.click(&format!("settings-accounts-edit-{}", AccountId::seed(7)));

    assert_eq!(ui.dialog(), Some("EditAccount"));
    assert_eq!(ui.account_form().unit.as_deref(), Some("btc"));
}

/// A click on the Delete Dialog's confirm, once the name is typed back, deletes in one commit.
#[gpui::test]
fn a_delete_confirmed_by_click_commits_once(app: &mut TestAppContext) {
    let mut ui = on_accounts(app);
    let before = ui.settings_accounts().account_names.len();

    ui.click(&format!("settings-accounts-delete-{}", AccountId::seed(1)));
    assert_eq!(ui.dialog(), Some("DeleteAccount"));
    ui.press("W a l l e t");
    let commits = ui.commits();
    ui.click("delete-account-confirm");

    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.commits(), commits + 1);
    let names = ui.settings_accounts().account_names;
    assert_eq!(names.len(), before - 1);
    assert!(!names.iter().any(|name| name == "Wallet"));
}
