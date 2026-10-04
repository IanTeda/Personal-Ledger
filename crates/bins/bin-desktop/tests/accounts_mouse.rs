//! The Add account dialog's form driven by clicks: focusing a text field, opening a select and
//! choosing an option, and the confirm button adding only a valid form. Only elements tagged with
//! `debug_selector` are clicked; state is read back from `Shell`.

mod common;

use bin_desktop::nav::Noun;
use common::Harness;
use gpui::TestAppContext;

fn add_dialog(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    ui.click("settings-index-Accounts");
    ui.click("settings-accounts-add");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddAccount"));
    ui
}

#[gpui::test]
fn clicking_a_text_field_focuses_it_and_typing_goes_there(app: &mut TestAppContext) {
    let mut ui = add_dialog(app);

    ui.click("add-account-number");
    assert_eq!(ui.account_form().focused, "AccountNumber");
    ui.press("4 2");
    assert_eq!(ui.account_form().account_number, "42");

    ui.click("add-account-name");
    assert_eq!(ui.account_form().focused, "Name");
    ui.press("a");
    let form = ui.account_form();
    assert_eq!(form.name, "a");
    assert_eq!(form.account_number, "42");
}

#[gpui::test]
fn clicking_a_select_opens_its_list_and_an_option_chooses_it(app: &mut TestAppContext) {
    let mut ui = add_dialog(app);
    let before = ui.account_form().institution;

    ui.click("add-account-institution");
    let form = ui.account_form();
    assert!(form.select_open);
    assert_eq!(form.focused, "Institution");

    ui.click("add-account-institution-option-1");
    let form = ui.account_form();
    assert!(!form.select_open);
    assert_ne!(form.institution, before);
}

#[gpui::test]
fn clicking_an_open_select_again_closes_it_without_a_change(app: &mut TestAppContext) {
    let mut ui = add_dialog(app);
    let before = ui.account_form().unit;

    ui.click("add-account-unit");
    assert!(ui.account_form().select_open);
    ui.click("add-account-unit");
    let form = ui.account_form();
    assert!(!form.select_open);
    assert_eq!(form.unit, before);
}

#[gpui::test]
fn choosing_cash_with_the_mouse_makes_the_institution_read_only(app: &mut TestAppContext) {
    let mut ui = add_dialog(app);

    ui.click("add-account-type");
    // Cash is the first type option.
    ui.click("add-account-type-option-0");
    assert_eq!(ui.account_form().account_type.as_deref(), Some("Cash"));

    ui.click("add-account-name");
    ui.click("add-account-institution");
    let form = ui.account_form();
    assert!(!form.select_open, "the Cash institution does not open");
    assert_eq!(form.focused, "Name");
}

#[gpui::test]
fn the_confirm_button_adds_only_a_valid_form(app: &mut TestAppContext) {
    let mut ui = add_dialog(app);
    let before = ui.settings().account_names.len();

    ui.click("add-account-confirm");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddAccount"));
    assert_eq!(ui.settings().account_names.len(), before);

    ui.press("r a i n y");
    ui.click("add-account-confirm");
    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert_eq!(page.account_names.len(), before + 1);
    assert!(page.account_names.iter().any(|name| name == "rainy"));
}
