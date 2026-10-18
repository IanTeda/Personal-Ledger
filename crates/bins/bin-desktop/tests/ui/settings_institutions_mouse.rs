//! The Settings Institutions and Units pages driven by clicks: the Institutions row and add
//! buttons, the Add institution dialog's chips, unit choice and buttons, the Add and Edit unit
//! dialogs' Type control and confirm button, and the Price Sources buttons. State is read back
//! from `Shell`.

use super::common;

use bin_desktop::{
    WindowSize,
    navigation::nav::{InputMode, Noun},
};
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
    ui.read(|shell| shell.status_message().map(str::to_string))
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
    let units_page = ui.settings_units();
    assert_eq!(ui.dialog(), Some("AddInstitution"));
    assert_eq!(ui.dialog_value("types").as_deref(), Some("Savings"));
    assert_eq!(
        ui.dialog_value("unit").as_deref(),
        units_page.unit_rows.first().map(|row| row.0.as_str())
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
    let units_page = ui.settings_units();
    assert_eq!(
        ui.dialog_value("unit").as_deref(),
        Some(units_page.unit_rows[1].0.as_str())
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
fn the_add_unit_type_control_and_confirm_button_add_the_unit(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Units");
    let before = ui.settings_units().unit_rows.len();

    ui.click("settings-add-unit");
    ui.click("add-unit-confirm");
    assert_eq!(ui.dialog(), Some("AddUnit"));
    ui.press("g o l d");
    ui.press("tab");
    ui.press("g o l d");
    ui.click("add-unit-type-4");
    assert_eq!(ui.dialog_value("kind").as_deref(), Some("Commodity"));
    ui.click("add-unit-confirm");
    let units_page = ui.settings_units();
    assert_eq!(ui.dialog(), None);
    assert_eq!(units_page.unit_rows.len(), before + 1);
    assert_eq!(
        units_page
            .unit_rows
            .last()
            .map(|row| (row.0.as_str(), row.2.as_str())),
        Some(("gold", "commodity"))
    );
}

#[gpui::test]
fn the_edit_unit_dialog_saves_a_changed_type_by_mouse(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Units");
    ui.resize_to(WindowSize::Large);
    let (code, name, _) = ui.settings_units().unit_rows[0].clone();

    row_action(&mut ui, "unit-row-0", "edit-unit");
    ui.click("edit-unit-type-1");
    assert_eq!(ui.dialog_value("kind").as_deref(), Some("Crypto"));
    ui.click("edit-unit-confirm");
    let units_page = ui.settings_units();
    assert_eq!(ui.dialog(), None);
    assert_eq!(units_page.unit_rows[0], (code, name, "crypto".to_string()));
}

#[gpui::test]
fn the_edit_unit_dialog_prefills_the_stored_type(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Units");
    ui.resize_to(WindowSize::Large);

    // vas is seeded as an Equity, which the old three-option selector could only show as Custom.
    row_action(&mut ui, "unit-row-2", "edit-unit");
    assert_eq!(ui.dialog_value("kind").as_deref(), Some("Equity"));
}

#[gpui::test]
fn test_price_source_flashes_not_yet_built(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Units");
    ui.resize_to(WindowSize::Large);

    row_action(&mut ui, "price-source-row-0", "test-source");
    assert_eq!(
        status(&mut ui).as_deref(),
        Some("test price source \u{2014} not yet built")
    );
    assert_eq!(ui.dialog(), None);
}
