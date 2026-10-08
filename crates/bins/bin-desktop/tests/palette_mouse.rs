//! The command palette opened with the mouse. Palette result rows have no click handler, so the
//! only mouse entry is the status line's `:` hint; running a result is a keyboard flow.

mod common;

use bin_desktop::navigation::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;

#[gpui::test]
fn clicking_the_command_hint_opens_the_palette(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    ui.click("hint-command");

    assert!(ui.read(|s| s.palette_snapshot()).is_some());
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Command);
}

#[gpui::test]
fn a_palette_opened_by_click_is_driven_by_keys(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.click("hint-command");

    ui.press("t r a n s a c t i o n s enter");

    assert_eq!(ui.noun(), Noun::Transactions);
    assert_eq!(ui.read(|s| s.palette_snapshot()), None);
}

#[gpui::test]
fn the_command_hint_does_nothing_outside_normal_mode(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("?");

    ui.click("hint-command");

    assert_eq!(ui.read(|s| s.palette_snapshot()), None);
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Help);
}
