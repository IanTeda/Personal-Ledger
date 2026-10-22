//! The Bills end-to-end smoke test: the Desktop's real start-up on a temporary SQLite file. Start,
//! Skip a bill, restart, see it still Skipped. Run with the other smoke tests at a parent ticket's
//! close and before a merge to `main` (`mise run end-to-end-ui smoke`).

use super::common;

use bin_desktop::navigation::nav::Noun;
use common::{Harness, config_for};
use gpui::TestAppContext;

fn skipped(ui: &mut Harness<'_>) -> usize {
    ui.bills()
        .rows
        .iter()
        .filter(|row| row.status == "Skipped")
        .count()
}

#[gpui::test]
fn a_skipped_bill_is_still_skipped_after_a_restart(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let config = config_for(&dir.path().join("ledger.pldb"));

    let (rows, skipped_before) = {
        let storage = bin_desktop::start_on(&config, common::today()).expect("a new Ledger starts");
        let writer = storage.writer.clone();
        let mut ui = Harness::with_storage(app, storage);
        ui.press("g w");
        assert_eq!(ui.noun(), Noun::Bills);
        let before = skipped(&mut ui);
        ui.press("s enter");
        assert_eq!(ui.dialog(), None, "the Skip Dialog confirmed");
        let after = skipped(&mut ui);
        assert_eq!(after, before + 1);
        let rows = ui.bills().rows;
        bin_desktop::quit(&writer);
        (rows, after)
    };

    let storage = bin_desktop::start_on(&config, common::today()).expect("the Ledger starts again");
    let mut ui = Harness::with_storage(app, storage);
    ui.press("g w");
    assert_eq!(skipped(&mut ui), skipped_before);
    assert_eq!(ui.bills().rows, rows);
}
