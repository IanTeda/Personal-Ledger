//! The Settings Institutions and Units pages' dialogs driven by keys once open: typing, `tab`
//! between the unit fields, `enter` to save and `escape` to cancel. The pages have no keys of
//! their own, so each dialog is opened with its tagged button. State is read back from `Shell`.

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

#[gpui::test]
fn add_unit_types_into_code_then_tab_moves_to_name_and_enter_saves(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Units");
    let before = ui.settings().unit_rows.len();

    ui.click("settings-add-unit");
    assert_eq!(ui.settings().unit_dialog_field.as_deref(), Some("Code"));
    ui.press("x y z");
    ui.press("tab");
    assert_eq!(ui.settings().unit_dialog_field.as_deref(), Some("Name"));
    ui.press("t e s t");
    let page = ui.settings();
    assert_eq!(page.dialog_name.as_deref(), Some("xyz"));
    assert_eq!(page.unit_dialog_name.as_deref(), Some("test"));
    ui.press("enter");
    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert_eq!(page.unit_rows.len(), before + 1);
    assert_eq!(
        page.unit_rows.last(),
        Some(&(
            "xyz".to_string(),
            "test".to_string(),
            "currency".to_string()
        ))
    );
}

#[gpui::test]
fn tab_cycles_back_to_code_and_backspace_trims_the_focused_field(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Units");

    ui.click("settings-add-unit");
    ui.press("a b");
    ui.press("tab tab");
    assert_eq!(ui.settings().unit_dialog_field.as_deref(), Some("Code"));
    ui.press("backspace");
    assert_eq!(ui.settings().dialog_name.as_deref(), Some("a"));
}

#[gpui::test]
fn add_unit_enter_does_nothing_until_code_and_name_are_filled(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Units");
    let before = ui.settings().unit_rows.len();

    ui.click("settings-add-unit");
    ui.press("a enter");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddUnit"));
    assert_eq!(ui.settings().unit_rows.len(), before);
}

#[gpui::test]
fn escape_discards_an_add_unit_draft(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Units");
    let before = ui.settings().unit_rows.len();

    ui.click("settings-add-unit");
    ui.press("a tab b");
    ui.press("escape");
    assert_eq!(ui.settings().dialog, None);
    assert_eq!(ui.settings().unit_rows.len(), before);
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Normal);
}

#[gpui::test]
fn edit_unit_saves_a_changed_name_with_enter(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Units");
    let (code, name, kind) = ui.settings().unit_rows[0].clone();

    ui.click("unit-edit-0");
    ui.press("tab");
    ui.press("x");
    ui.press("enter");
    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert_eq!(page.unit_rows[0], (code, format!("{name}x"), kind));
}

#[gpui::test]
fn add_institution_types_a_name_and_enter_adds_the_row(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");
    let before = ui.settings().institutions.len();

    ui.click("settings-add-institution");
    ui.press("b a n k");
    assert_eq!(ui.settings().dialog_name.as_deref(), Some("bank"));
    ui.press("backspace");
    assert_eq!(ui.settings().dialog_name.as_deref(), Some("ban"));
    ui.press("tab");
    ui.press("enter");
    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert_eq!(page.institutions.len(), before + 1);
    assert_eq!(
        page.institutions.last().map(|row| row.0.as_str()),
        Some("ban")
    );
}

#[gpui::test]
fn add_institution_enter_does_nothing_without_a_name(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");
    let before = ui.settings().institutions.len();

    ui.click("settings-add-institution");
    ui.press("enter");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddInstitution"));
    assert_eq!(ui.settings().institutions.len(), before);
    ui.press("escape");
    assert_eq!(ui.settings().dialog, None);
}
