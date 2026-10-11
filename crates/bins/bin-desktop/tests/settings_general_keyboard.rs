//! The Settings View's own keys on the index and the General page: the index's ends, `/` with no
//! Search, and `Esc` on the index. State is read back from `Shell`.

mod common;

use bin_desktop::navigation::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;

fn on_settings(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    ui
}

#[gpui::test]
fn shift_g_and_g_g_jump_to_the_index_ends(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    ui.press("shift-g");
    assert_eq!(ui.settings().page, "About");
    ui.press("g g");
    assert_eq!(ui.settings().page, "General");
}

#[gpui::test]
fn slash_is_inert_on_settings(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    ui.press("/");
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Normal);
    assert_eq!(ui.settings().page, "General");
}

#[gpui::test]
fn escape_on_the_index_stays_on_settings(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    ui.press("l");
    assert!(ui.settings().page_focused);
    ui.press("escape escape");
    let page = ui.settings();
    assert!(!page.page_focused);
    assert_eq!(page.page, "General");
    assert_eq!(ui.noun(), Noun::Settings);
}
