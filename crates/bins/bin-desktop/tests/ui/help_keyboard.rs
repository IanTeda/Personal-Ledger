//! The `?` Help Dialog driven by keys: `?` opens it over the View and `Esc` closes it. State is
//! read back from `Root`.

use super::common;

use bin_desktop::navigation::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;

fn mode(ui: &mut Harness<'_>) -> InputMode {
    ui.read(|shell| shell.nav().mode())
}

#[gpui::test]
fn help_opens_with_question_mark_and_escape_closes_it(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    ui.press("?");
    assert_eq!(mode(&mut ui), InputMode::Dialog);
    assert!(ui.read(|s| s.help_open()));
    ui.press("escape");
    assert_eq!(mode(&mut ui), InputMode::Normal);
    assert_eq!(ui.noun(), Noun::Dashboard);
}
