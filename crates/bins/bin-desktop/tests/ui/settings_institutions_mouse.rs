//! The Settings Institutions page driven by clicks: the Institutions row and add buttons, and the
//! Add institution dialog's chips, unit choice and buttons. State is read back from `Root`.

use super::common;

use bin_desktop::navigation::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;

fn on_page<'a>(app: &'a mut TestAppContext, page: &str) -> Harness<'a> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    ui.click(&format!("settings-index-{page}"));
    assert_eq!(ui.settings().page, page);
    ui
}

fn status(ui: &mut Harness<'_>) -> Option<String> {
    ui.read(|shell| shell.status_message())
}

/// Right-clicks the row and picks an entry of its context menu: the rows carry no buttons.
fn row_action(ui: &mut Harness<'_>, row: &str, action: &str) {
    ui.right_click(row);
    ui.click(&format!("settings-units-menu-{action}"));
}

#[gpui::test]
fn institution_edit_and_delete_buttons_flash_a_not_yet_built_message(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");
    let before = ui.settings_institutions().institutions;

    ui.click("institution-edit-0");
    assert_eq!(
        status(&mut ui).as_deref(),
        Some("edit institution \u{2014} not yet built")
    );
    ui.click("institution-delete-1");
    assert_eq!(
        status(&mut ui).as_deref(),
        Some("delete institution \u{2014} not yet built")
    );
    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.settings_institutions().institutions, before);
}

#[gpui::test]
fn the_add_institution_dialog_seeds_savings_and_the_first_unit(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");

    ui.click("settings-add-institution");
    let unit_rows = ui.unit_rows();
    assert_eq!(ui.dialog(), Some("AddInstitution"));
    assert_eq!(ui.dialog_value("types").as_deref(), Some("Savings"));
    assert_eq!(
        ui.dialog_value("unit").as_deref(),
        unit_rows.first().map(|row| row.0.as_str())
    );
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Dialog);
    ui.click("add-institution-cancel");
    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Normal);
}

#[gpui::test]
fn account_type_chips_toggle_and_the_unit_choice_changes(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");

    ui.click("settings-add-institution");
    ui.click("add-institution-chip-offset");
    assert_eq!(ui.dialog_value("types").as_deref(), Some("Savings,Offset"));
    ui.click("add-institution-chip-savings");
    assert_eq!(ui.dialog_value("types").as_deref(), Some("Offset"));
    ui.click("add-institution-unit-1");
    let unit_rows = ui.unit_rows();
    assert_eq!(
        ui.dialog_value("unit").as_deref(),
        Some(unit_rows[1].0.as_str())
    );
}

#[gpui::test]
fn the_add_institution_button_needs_a_name_and_a_type_then_adds_the_row(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");
    let before = ui.settings_institutions().institutions.len();

    ui.click("settings-add-institution");
    ui.click("add-institution-confirm");
    assert_eq!(ui.dialog(), Some("AddInstitution"));
    ui.press("n e w");
    ui.click("add-institution-chip-offset");
    ui.click("add-institution-confirm");
    let institutions_page = ui.settings_institutions();
    assert_eq!(ui.dialog(), None);
    assert_eq!(institutions_page.institutions.len(), before + 1);
    assert_eq!(
        institutions_page.institutions.last(),
        Some(&("new".to_string(), "savings \u{b7} offset".to_string()))
    );
}

#[gpui::test]
fn an_institution_with_no_account_type_cannot_be_added(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");
    let before = ui.settings_institutions().institutions.len();

    ui.click("settings-add-institution");
    ui.press("n e w");
    ui.click("add-institution-chip-savings");
    ui.click("add-institution-confirm");
    assert_eq!(ui.dialog(), Some("AddInstitution"));
    assert_eq!(ui.settings_institutions().institutions.len(), before);
}

#[gpui::test]
fn a_row_click_selects_its_institution_and_a_row_button_leaves_the_selection_on_that_row(
    app: &mut TestAppContext,
) {
    let mut ui = on_page(app, "Institutions");
    let names: Vec<String> = ui
        .settings_institutions()
        .institutions
        .into_iter()
        .map(|row| row.0)
        .collect();

    ui.click("settings-institutions-row-2");
    assert_eq!(
        ui.settings_institutions().selected_institution.as_ref(),
        names.get(2)
    );
    ui.click("institution-delete-4");
    assert_eq!(
        ui.settings_institutions().selected_institution.as_ref(),
        names.get(4)
    );
    assert_eq!(ui.dialog(), None);
}
