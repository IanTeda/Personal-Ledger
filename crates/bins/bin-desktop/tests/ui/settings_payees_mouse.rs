//! The Settings Payees page driven by clicks: the add button, row selection, the row actions and
//! the dialogs' buttons. Only elements tagged with `debug_selector` are clicked.

use super::common;
use lib_core::PayeeId;

use bin_desktop::navigation::nav::InputMode;
use common::Harness;
use gpui::TestAppContext;

fn on_payees(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    ui.click("settings-index-Payees");
    assert_eq!(ui.settings().page, "Payees");
    ui
}

#[gpui::test]
fn the_add_button_opens_the_dialog_and_cancel_closes_it(app: &mut TestAppContext) {
    let mut ui = on_payees(app);

    ui.click("settings-payees-add");
    assert_eq!(ui.dialog(), Some("AddPayee"));
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Dialog);
    ui.click("add-payee-cancel");
    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Normal);
}

#[gpui::test]
fn the_add_dialog_confirm_adds_and_selects_the_payee(app: &mut TestAppContext) {
    let mut ui = on_payees(app);
    let before = ui.settings_payees().payee_names.len();

    ui.click("settings-payees-add");
    ui.click("add-payee-confirm");
    assert_eq!(ui.dialog(), Some("AddPayee"));
    ui.press("z o o");
    ui.click("add-payee-confirm");
    let payees_page = ui.settings_payees();
    assert_eq!(ui.dialog(), None);
    assert_eq!(payees_page.payee_names.len(), before + 1);
    assert_eq!(payees_page.selected_payee.as_deref(), Some("zoo"));
}

#[gpui::test]
fn clicking_a_row_selects_it_and_focuses_the_page(app: &mut TestAppContext) {
    let mut ui = on_payees(app);
    assert!(!ui.settings().page_focused);

    ui.click(&format!("settings-payees-row-{}", PayeeId::seed(8)));
    assert_eq!(
        ui.settings_payees().selected_payee.as_deref(),
        Some("J Smith")
    );
    assert!(ui.settings().page_focused);
}

#[gpui::test]
fn the_edit_action_selects_the_row_and_opens_its_dialog_with_the_name(app: &mut TestAppContext) {
    let mut ui = on_payees(app);

    ui.click(&format!("settings-payees-edit-{}", PayeeId::seed(8)));
    assert_eq!(ui.dialog(), Some("EditPayee"));
    assert_eq!(ui.dialog_value("name").as_deref(), Some("J Smith"));
    assert_eq!(
        ui.settings_payees().selected_payee.as_deref(),
        Some("J Smith")
    );
    ui.click("edit-payee-cancel");
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn the_delete_action_deactivates_a_used_payee_once_its_name_is_typed(app: &mut TestAppContext) {
    let mut ui = on_payees(app);
    let before = ui.settings_payees().payee_names;

    ui.click(&format!("settings-payees-delete-{}", PayeeId::seed(1)));
    assert_eq!(ui.dialog(), Some("DeletePayee"));
    ui.click("delete-payee-cancel");
    assert_eq!(ui.dialog(), None);
    ui.click(&format!("settings-payees-delete-{}", PayeeId::seed(1)));
    ui.click("delete-payee-confirm");
    assert_eq!(ui.dialog(), Some("DeletePayee"));
    ui.press("W o o l w o r t h s");
    ui.click("delete-payee-confirm");
    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.settings_payees().payee_names, before);
    assert_eq!(
        ui.toasts().history.first().map(|toast| toast.text.clone()),
        Some("Deactivated payee Woolworths".to_string())
    );
}

#[gpui::test]
fn a_rule_and_a_default_category_are_added_by_clicks(app: &mut TestAppContext) {
    let mut ui = on_payees(app);

    ui.click("settings-payees-add");
    ui.press("z o o");
    ui.click("add-payee-category");
    ui.click("add-payee-category-option-1");
    assert_eq!(ui.dialog_value("category").as_deref(), Some("Rent"));
    ui.click("add-payee-rule-input");
    ui.press("z o o x");
    ui.click("add-payee-rule-add");
    assert_eq!(ui.dialog_value("rules").as_deref(), Some("ZOOX"));
    ui.click("add-payee-rule-remove-0");
    assert_eq!(ui.dialog_value("rules").as_deref(), Some(""));
    ui.click("add-payee-confirm");
    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.settings_payees().selected_payee.as_deref(), Some("zoo"));
}

#[gpui::test]
fn a_documents_payee_link_lands_on_the_selected_payee(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.click("primary-rail-Documents");
    // Rows below the fold have no on-screen bounds to click, so reaching one is set-up.
    for _ in 0..200 {
        if ui.documents().selected_file.as_deref() == Some("aami-car-policy.pdf") {
            break;
        }
        ui.press("j");
    }

    ui.click("documents-link-0");

    assert_eq!(ui.settings().page, "Payees");
    assert_eq!(ui.settings_payees().selected_payee.as_deref(), Some("AAMI"));
    ui.click(&format!("settings-payees-edit-{}", PayeeId::seed(22)));
    assert_eq!(ui.dialog_value("name").as_deref(), Some("AAMI"));
}
