//! The Bills end-to-end smoke tests: the Desktop's real start-up on a temporary SQLite file. Start,
//! Skip a bill or pay one directly, restart, see it still Skipped or Paid, and the paid bill's
//! Transaction in Transactions. Run with the other smoke tests at a parent ticket's close and
//! before a merge to `main` (`mise run end-to-end-ui smoke`).

#![expect(
    clippy::expect_used,
    reason = "test support: a Telstra Internet row the stub doesn't seed should fail the test loudly"
)]

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

/// Where the Telstra Internet row sits, wherever the Schedule's order has put it.
fn telstra_row(ui: &mut Harness<'_>) -> usize {
    ui.bills()
        .rows
        .iter()
        .position(|row| row.plan == "Telstra Internet")
        .expect("Telstra Internet is due this month")
}

/// Opens the Bills page with the Telstra Internet row selected.
fn on_telstra(ui: &mut Harness<'_>) {
    ui.press("g w");
    assert_eq!(ui.noun(), Noun::Bills);
    let row = telstra_row(ui);
    ui.press("g g");
    for _ in 0..row {
        ui.press("j");
    }
}

fn telstra_status(ui: &mut Harness<'_>) -> String {
    let row = telstra_row(ui);
    ui.bills().rows[row].status.clone()
}

/// How many Transactions the Transactions page shows paid to Telstra.
fn telstra_transactions(ui: &mut Harness<'_>) -> usize {
    ui.press("g l");
    assert_eq!(ui.noun(), Noun::Transactions);
    ui.transactions()
        .payees
        .iter()
        .filter(|payee| *payee == "Telstra")
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

#[gpui::test]
fn a_bill_paid_directly_is_still_paid_with_its_transaction_after_a_restart(
    app: &mut TestAppContext,
) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let config = config_for(&dir.path().join("ledger.pldb"));

    let (rows, telstra_paid) = {
        let storage = bin_desktop::start_on(&config, common::today()).expect("a new Ledger starts");
        let writer = storage.writer.clone();
        let mut ui = Harness::with_storage(app, storage);
        let telstra_before = telstra_transactions(&mut ui);
        on_telstra(&mut ui);
        assert_ne!(telstra_status(&mut ui), "Paid");

        ui.press("p");
        assert_eq!(ui.bills().dialog.as_deref(), Some("Pay"));
        // Leave Match for Pay it directly, whose amount and date default from the Plan.
        ui.press("right");
        assert_eq!(ui.bills().pay_mode.as_deref(), Some("Direct"));
        ui.press("enter");
        assert_eq!(ui.bills().dialog, None, "the Pay Dialog confirmed");
        assert_eq!(telstra_status(&mut ui), "Paid");

        let rows = ui.bills().rows;
        let telstra_paid = telstra_transactions(&mut ui);
        assert_eq!(
            telstra_paid,
            telstra_before + 1,
            "Pay wrote one Transaction"
        );
        bin_desktop::quit(&writer);
        (rows, telstra_paid)
    };

    let storage = bin_desktop::start_on(&config, common::today()).expect("the Ledger starts again");
    let mut ui = Harness::with_storage(app, storage);
    assert_eq!(telstra_transactions(&mut ui), telstra_paid);
    // Move off the top row, where the new Transaction (dated today, newest first) sits, so only the
    // handoff can select it again.
    ui.press("j j j");
    assert_eq!(ui.transactions().selected, 3);
    on_telstra(&mut ui);
    assert_eq!(telstra_status(&mut ui), "Paid");
    assert_eq!(ui.bills().rows, rows);

    // Enter on the Paid row opens its Transaction, selected, in Transactions.
    ui.press("enter");
    assert_eq!(ui.noun(), Noun::Transactions);
    let page = ui.transactions();
    assert_eq!(page.selected, 0);
    assert_eq!(page.payees[page.selected], "Telstra");
}
