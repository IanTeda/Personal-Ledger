//! The command palette opened with the mouse. Palette result rows have no click handler, so the
//! only mouse entry is the status line's `:` hint; running a result is a keyboard flow.

use super::common;

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
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Dialog);
}

#[gpui::test]
fn a_palette_opened_by_click_lists_each_command_with_its_live_key(app: &mut TestAppContext) {
    let mut ui = Harness::with_keybindings(app, common::overriding("jump_dashboard", "g h"));

    ui.click("hint-command");

    let palette = ui.read(|s| s.palette_snapshot()).expect("open");
    let key = |name: &str| {
        let row = palette.matches.iter().position(|matched| *matched == name);
        row.map(|row| palette.keys[row].clone())
    };
    assert_eq!(key("dashboard").as_deref(), Some("g h"));
    assert_eq!(key("units set-base").as_deref(), Some("B"));
    assert_eq!(key("toasts").as_deref(), Some("\u{2014}"));
}

#[gpui::test]
fn a_module_command_runs_from_a_palette_opened_by_click(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.click("hint-command");

    ui.press("t a g s space m e r g e enter");

    assert_eq!(ui.noun(), Noun::Settings);
    assert_eq!(ui.dialog(), Some("MergeTags"));
}
