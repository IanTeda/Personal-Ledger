//! The Settings About page driven by keys. About has no controls, but `l` and `enter` on the
//! index open it like any page, the index being Settings' resting view. The checks cover reaching the page, the
//! last of THIS DEVICE, the keys that must leave it unchanged, `j` crossing on into THIS LEDGER,
//! and `esc` returning focus to the index from a page that has it. State is read back from `Root`.

use super::common;

use bin_desktop::navigation::nav::Noun;
use common::Harness;
use gpui::TestAppContext;

fn on_about(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    ui.press("j j j");
    assert_eq!(ui.settings_frame().page, "About");
    ui
}

#[gpui::test]
fn j_down_the_index_swaps_to_about_with_focus_on_the_index(app: &mut TestAppContext) {
    let mut ui = on_about(app);

    let page = ui.settings_frame();
    assert_eq!(page.page, "About");
    assert!(!page.page_focused);
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn the_about_page_shows_its_version_line(app: &mut TestAppContext) {
    let mut ui = on_about(app);
    assert!(!ui.is_drawn("settings-about-version"), "the index shows");

    ui.press("enter");

    assert!(ui.is_drawn("settings-about-version"));
}

#[gpui::test]
fn l_or_enter_open_about_though_it_has_nothing_to_focus(app: &mut TestAppContext) {
    for open in ["l", "enter"] {
        let mut ui = on_about(app);
        let toasts_before = ui.toasts();

        ui.press(open);

        let page = ui.settings_frame();
        assert_eq!(page.page, "About");
        assert!(page.page_focused, "{open} opens About");
        assert_eq!(page.status_message, None);
        assert_eq!(ui.toasts(), toasts_before);

        ui.press("escape");
        assert!(
            !ui.settings_frame().page_focused,
            "esc returns to the index"
        );
    }
}

#[gpui::test]
fn j_from_about_crosses_from_this_device_into_this_ledger(app: &mut TestAppContext) {
    let mut ui = on_about(app);
    let toasts_before = ui.toasts();

    ui.press("j");
    let page = ui.settings_frame();
    assert_eq!(page.page, "Ledger");
    assert!(!page.page_focused);
    ui.press("k");
    assert_eq!(ui.settings_frame().page, "About");
    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.toasts(), toasts_before);
    assert_eq!(ui.noun(), Noun::Settings);
}

#[gpui::test]
fn escape_returns_focus_to_the_index_before_reaching_about(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("g s");
    ui.press("l");
    assert!(ui.settings_frame().page_focused);

    ui.press("escape");
    let after_escape = ui.settings_frame();
    assert!(!after_escape.page_focused);
    assert_eq!(after_escape.page, "General");

    ui.press("j j j");
    let page = ui.settings_frame();
    assert_eq!(page.page, "About");
    assert!(!page.page_focused);
    ui.press("escape");
    assert_eq!(ui.noun(), Noun::Settings);
    assert!(!ui.settings_frame().page_focused);
}
