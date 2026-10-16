//! The Dashboard driven by keys: it is the start page, lists the Bills needing attention, scrolls
//! on movement keys and takes `/` into a Search with nothing to type into. State is read back from
//! `Shell`.

use super::common;

use bin_desktop::navigation::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;

fn mode(ui: &mut Harness<'_>) -> InputMode {
    ui.read(|shell| shell.nav().mode())
}

#[gpui::test]
fn the_dashboard_is_the_start_page_and_lists_bills_needing_attention(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    assert_eq!(ui.noun(), Noun::Dashboard);

    let bills = ui.dashboard_bills();
    assert!(!bills.is_empty(), "the stub ledger has Bills to attend to");
    assert!(bills.iter().all(|bill| !bill.plan.is_empty()));
}

#[gpui::test]
fn movement_keys_scroll_the_dashboard_without_leaving_it(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    ui.press("j j k g g G");

    assert_eq!(ui.noun(), Noun::Dashboard);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn slash_enters_search_on_the_dashboard_and_escape_leaves_it(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    ui.press("/");
    assert_eq!(mode(&mut ui), InputMode::Search);
    ui.press("x");
    assert_eq!(
        mode(&mut ui),
        InputMode::Search,
        "the Dashboard has no box to type into"
    );
    ui.press("escape");

    assert_eq!(mode(&mut ui), InputMode::Normal);
    assert_eq!(ui.noun(), Noun::Dashboard);
}
