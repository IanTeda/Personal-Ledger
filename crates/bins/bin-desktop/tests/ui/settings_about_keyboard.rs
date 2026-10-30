//! The Settings About page driven by keys. About has no controls, so `l` and `enter` on the
//! index are a quiet no-op and focus stays on the index. The checks cover reaching the page
//! with `shift-g`, the keys that must leave it unchanged, and `esc` returning focus to the index
//! from a page that has it. State is read back from `Root`.

use super::common;

use bin_desktop::navigation::nav::Noun;
use common::Harness;
use gpui::TestAppContext;

fn on_about(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    ui.press("shift-g");
    assert_eq!(ui.settings_frame().page, "About");
    ui
}

#[gpui::test]
fn shift_g_from_the_index_swaps_to_about_with_focus_on_the_index(app: &mut TestAppContext) {
    let mut ui = on_about(app);

    let page = ui.settings_frame();
    assert_eq!(page.page, "About");
    assert!(!page.page_focused);
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn the_about_page_shows_its_version_line(app: &mut TestAppContext) {
    let mut ui = on_about(app);

    assert!(ui.is_drawn("settings-about-version"));
}

#[gpui::test]
fn stepping_into_about_is_a_quiet_no_op(app: &mut TestAppContext) {
    let mut ui = on_about(app);
    let toasts_before = ui.toasts();

    ui.press("l");
    ui.press("enter");

    let page = ui.settings_frame();
    assert_eq!(page.page, "About");
    assert!(!page.page_focused);
    assert_eq!(page.status_message, None);
    assert_eq!(ui.toasts(), toasts_before);
}

#[gpui::test]
fn movement_keys_at_the_bottom_of_the_index_leave_about_unchanged(app: &mut TestAppContext) {
    let mut ui = on_about(app);
    let toasts_before = ui.toasts();

    ui.press("j j shift-g");

    let page = ui.settings_frame();
    assert_eq!(page.page, "About");
    assert!(!page.page_focused);
    assert_eq!(ui.dialog(), None);
    assert_eq!(page.status_message, None);
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

    ui.press("shift-g");
    let page = ui.settings_frame();
    assert_eq!(page.page, "About");
    assert!(!page.page_focused);
    ui.press("escape");
    assert_eq!(ui.noun(), Noun::Settings);
    assert!(!ui.settings_frame().page_focused);
}
