//! The Settings Institutions and Units pages driven by clicks: the Institutions row and add
//! buttons, the Add institution dialog's chips, unit choice and buttons, the Add and Edit unit
//! dialogs' Type control and confirm button, and the Price Sources buttons. State is read back
//! from `Shell`.

mod common;

use bin_desktop::nav::{InputMode, Noun};
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

#[gpui::test]
fn institution_edit_and_delete_buttons_flash_a_not_yet_built_message(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");
    let before = ui.settings().institutions;

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
    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert_eq!(page.institutions, before);
}

#[gpui::test]
fn the_add_institution_dialog_seeds_savings_and_the_first_unit(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");

    ui.click("settings-add-institution");
    let page = ui.settings();
    assert_eq!(page.dialog.as_deref(), Some("AddInstitution"));
    assert_eq!(
        page.institution_dialog_types,
        Some(vec!["Savings".to_string()])
    );
    assert_eq!(
        page.institution_dialog_unit.as_deref(),
        page.unit_rows.first().map(|row| row.0.as_str())
    );
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Dialog);
    ui.click("add-institution-cancel");
    assert_eq!(ui.settings().dialog, None);
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Normal);
}

#[gpui::test]
fn account_type_chips_toggle_and_the_unit_choice_changes(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");

    ui.click("settings-add-institution");
    ui.click("add-institution-chip-offset");
    assert_eq!(
        ui.settings().institution_dialog_types,
        Some(vec!["Savings".to_string(), "Offset".to_string()])
    );
    ui.click("add-institution-chip-savings");
    assert_eq!(
        ui.settings().institution_dialog_types,
        Some(vec!["Offset".to_string()])
    );
    ui.click("add-institution-unit-1");
    let page = ui.settings();
    assert_eq!(
        page.institution_dialog_unit.as_deref(),
        Some(page.unit_rows[1].0.as_str())
    );
}

#[gpui::test]
fn the_add_institution_button_needs_a_name_and_a_type_then_adds_the_row(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");
    let before = ui.settings().institutions.len();

    ui.click("settings-add-institution");
    ui.click("add-institution-confirm");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddInstitution"));
    ui.press("n e w");
    ui.click("add-institution-chip-offset");
    ui.click("add-institution-confirm");
    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert_eq!(page.institutions.len(), before + 1);
    assert_eq!(
        page.institutions.last(),
        Some(&("new".to_string(), "savings \u{b7} offset".to_string()))
    );
}

#[gpui::test]
fn an_institution_with_no_account_type_cannot_be_added(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");
    let before = ui.settings().institutions.len();

    ui.click("settings-add-institution");
    ui.press("n e w");
    ui.click("add-institution-chip-savings");
    ui.click("add-institution-confirm");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddInstitution"));
    assert_eq!(ui.settings().institutions.len(), before);
}

#[gpui::test]
fn the_add_unit_type_control_and_confirm_button_add_the_unit(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Units");
    let before = ui.settings().unit_rows.len();

    ui.click("settings-add-unit");
    ui.click("add-unit-confirm");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddUnit"));
    ui.press("g o l d");
    ui.press("tab");
    ui.press("g o l d");
    ui.click("add-unit-type-2");
    assert_eq!(ui.settings().unit_dialog_kind.as_deref(), Some("Custom"));
    ui.click("add-unit-confirm");
    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert_eq!(page.unit_rows.len(), before + 1);
    assert_eq!(
        page.unit_rows
            .last()
            .map(|row| (row.0.as_str(), row.2.as_str())),
        Some(("gold", "custom"))
    );
}

#[gpui::test]
fn the_edit_unit_dialog_saves_a_changed_type_by_mouse(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Units");
    let (code, name, _) = ui.settings().unit_rows[0].clone();

    ui.click("unit-edit-0");
    ui.click("edit-unit-type-1");
    assert_eq!(
        ui.settings().unit_dialog_kind.as_deref(),
        Some("Cryptocurrency")
    );
    ui.click("edit-unit-confirm");
    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert_eq!(
        page.unit_rows[0],
        (code, name, "cryptocurrency".to_string())
    );
}

#[gpui::test]
fn the_price_source_buttons_flash_not_yet_built_messages(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Units");

    ui.click("price-source-test-0");
    assert_eq!(
        status(&mut ui).as_deref(),
        Some("test price source \u{2014} not yet built")
    );
    ui.click("price-source-edit-0");
    assert_eq!(
        status(&mut ui).as_deref(),
        Some("edit price source \u{2014} not yet built")
    );
    ui.click("price-source-delete-1");
    assert_eq!(
        status(&mut ui).as_deref(),
        Some("delete price source \u{2014} not yet built")
    );
    ui.click("settings-add-price-source");
    assert_eq!(
        status(&mut ui).as_deref(),
        Some("add price source \u{2014} not yet built")
    );
    assert_eq!(ui.settings().dialog, None);
}
