//! The `?` Help Dialog driven by clicks: its close button. Only elements tagged with
//! `debug_selector` are clicked; state is read back from `Root`.

use super::common;

use bin_desktop::navigation::nav::InputMode;
use common::Harness;
use gpui::TestAppContext;

#[gpui::test]
fn the_help_close_button_leaves_help_mode(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("?");
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Dialog);
    assert!(ui.read(|s| s.help_open()));

    ui.click("help-close");
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Normal);
}
