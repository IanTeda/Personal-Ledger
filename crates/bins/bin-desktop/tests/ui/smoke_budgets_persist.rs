//! The Budgets end-to-end smoke test: the Desktop's real start-up on a temporary SQLite file. Add a
//! Budget, restart, and see it in the Switcher, read from the Budgets adapter. Run with the other
//! smoke tests at a parent ticket's close and before a merge to `main` (`mise run end-to-end-ui
//! smoke`).

#![expect(
    clippy::expect_used,
    reason = "test support: a Ledger that won't start should fail the test loudly"
)]

use super::common;

use bin_desktop::navigation::nav::Noun;
use common::{Harness, config_for};
use gpui::TestAppContext;

/// The Budget names the Switcher lists, closing it again.
fn budget_names(ui: &mut Harness<'_>) -> Vec<String> {
    ui.press("g b");
    assert_eq!(ui.noun(), Noun::Budgets);
    ui.press("shift-b");
    let names = ui.budgets().switcher_names.expect("the Switcher is open");
    ui.press("escape");
    names
}

#[gpui::test]
fn an_added_budget_is_there_after_a_restart(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let config = config_for(&dir.path().join("ledger.pldb"));

    let names = {
        let storage = bin_desktop::start_on(&config, common::today()).expect("a new Ledger starts");
        let writer = storage.writer.clone();
        let mut ui = Harness::with_storage(app, storage);
        assert!(!budget_names(&mut ui).contains(&"trip".to_string()));
        ui.press("n t r i p enter");
        assert_eq!(ui.budgets().dialog, None, "the New budget Dialog confirmed");
        let names = budget_names(&mut ui);
        assert!(names.contains(&"trip".to_string()));
        bin_desktop::quit(&writer);
        names
    };

    let storage = bin_desktop::start_on(&config, common::today()).expect("the Ledger starts again");
    let mut ui = Harness::with_storage(app, storage);
    assert_eq!(budget_names(&mut ui), names);
}
