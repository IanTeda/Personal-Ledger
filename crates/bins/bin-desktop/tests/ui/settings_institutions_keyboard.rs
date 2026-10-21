//! The Settings Institutions page's dialog driven by keys once open: typing, `enter` to save and
//! `escape` to cancel. The page has no keys of its own, so the dialog is opened with its tagged
//! button. State is read back from `Shell`.

use super::common;

use bin_desktop::navigation::nav::Noun;
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
fn add_institution_types_a_name_and_enter_adds_the_row(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");
    let before = ui.settings_institutions().institutions.len();

    ui.click("settings-add-institution");
    ui.press("b a n k");
    assert_eq!(ui.dialog_value("name").as_deref(), Some("bank"));
    ui.press("backspace");
    assert_eq!(ui.dialog_value("name").as_deref(), Some("ban"));
    ui.press("tab");
    ui.press("enter");
    let institutions_page = ui.settings_institutions();
    assert_eq!(ui.dialog(), None);
    assert_eq!(institutions_page.institutions.len(), before + 1);
    assert_eq!(
        institutions_page
            .institutions
            .last()
            .map(|row| row.0.as_str()),
        Some("ban")
    );
}

#[gpui::test]
fn add_institution_enter_does_nothing_without_a_name(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");
    let before = ui.settings_institutions().institutions.len();

    ui.click("settings-add-institution");
    ui.press("enter");
    assert_eq!(ui.dialog(), Some("AddInstitution"));
    assert_eq!(ui.settings_institutions().institutions.len(), before);
    ui.press("escape");
    assert_eq!(ui.dialog(), None);
}
