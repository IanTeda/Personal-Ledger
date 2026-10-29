//! The Pay Dialog driven by keys, on the in-memory ports: the Bills page opens it on its selected
//! row, each key reaches the `lib-bill-payments` Presenter, and what it changes is drawn. The rules
//! behind each (pre-select, refusals, the amount and date checks) are the Presenter's tests in
//! `lib-bill-payments`; these check the wiring, the Bills page after a confirm and a failed save.

#![expect(
    clippy::expect_used,
    reason = "test support: a Telstra Internet row the stub doesn't seed should fail the test loudly"
)]

use super::common;

use bin_desktop::WindowSize;
use bin_desktop::navigation::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;

fn on_bills(ui: &mut Harness<'_>) {
    ui.press("g w");
    assert_eq!(ui.noun(), Noun::Bills);
}

/// The Bills page with the Telstra Internet row selected: it has Match candidates.
fn on_telstra(ui: &mut Harness<'_>) {
    on_bills(ui);
    let row = ui
        .bills()
        .rows
        .iter()
        .position(|row| row.plan == "Telstra Internet")
        .expect("Telstra Internet is due this month");
    for _ in 0..row {
        ui.press("j");
    }
}

/// The status of the Telstra Internet row, wherever the Schedule's order has put it.
fn telstra_status(ui: &mut Harness<'_>) -> String {
    ui.bills()
        .rows
        .into_iter()
        .find(|row| row.plan == "Telstra Internet")
        .expect("Telstra Internet is due this month")
        .status
}

fn paid_rows(ui: &mut Harness<'_>) -> usize {
    ui.bills()
        .rows
        .iter()
        .filter(|row| row.status == "Paid")
        .count()
}

#[gpui::test]
fn p_opens_the_dialog_titled_with_its_plan_in_dialog_mode(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    on_telstra(&mut ui);

    ui.press("p");

    assert_eq!(ui.bills().dialog.as_deref(), Some("Pay"));
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Dialog);
    assert!(
        ui.dialog_value("title")
            .is_some_and(|title| title.starts_with("Pay \u{2014} Telstra Internet")),
        "{:?}",
        ui.dialog_value("title")
    );
    assert!(ui.is_drawn("pay-bill-candidate-0"));
    assert!(ui.is_drawn("pay-bill-none-of-these"));
    let hints: Vec<String> = ui
        .status()
        .expect("the Dialog has hints")
        .hints
        .into_iter()
        .map(|(key, _)| key)
        .collect();
    assert_eq!(hints, ["esc", "enter", "j/k", "\u{2190}/\u{2192}"]);
}

#[gpui::test]
fn enter_on_the_preselected_candidate_matches_it(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    on_telstra(&mut ui);

    ui.press("p");
    let page = ui.bills();
    assert_eq!(page.pay_mode.as_deref(), Some("Match"));
    assert!(page.pay_candidates.unwrap_or(0) > 0);
    ui.press("enter");

    assert_eq!(ui.bills().dialog, None);
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Normal);
    assert_eq!(paid_rows(&mut ui), 2);
}

#[gpui::test]
fn paying_directly_marks_the_row_paid_with_a_transaction_to_open(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    on_telstra(&mut ui);

    ui.press("p");
    // Leave Match for Pay it directly, whose amount and date default from the Plan.
    ui.press("right");
    assert_eq!(ui.bills().pay_mode.as_deref(), Some("Direct"));
    assert!(ui.is_drawn("pay-bill-amount"));
    for _ in 0..10 {
        ui.press("backspace");
    }
    ui.press("4 2 . 5");
    assert_eq!(ui.dialog_value("amount").as_deref(), Some("42.5"));
    ui.press("enter");

    assert_eq!(ui.bills().dialog, None);
    assert_eq!(telstra_status(&mut ui), "Paid");
    // The new Transaction is the row's: Enter on the Paid row, now sorted below the open ones,
    // opens it.
    let row = ui
        .bills()
        .rows
        .iter()
        .position(|row| row.plan == "Telstra Internet")
        .expect("Telstra Internet is due this month");
    ui.press("g g");
    for _ in 0..row {
        ui.press("j");
    }
    ui.press("enter");
    assert_eq!(ui.noun(), Noun::Transactions);
}

#[gpui::test]
fn j_and_k_choose_a_row_and_enter_on_none_of_these_switches_panel(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    on_telstra(&mut ui);
    ui.press("p");
    let candidates = ui.bills().pay_candidates.expect("the Pay Dialog is open");

    for _ in 0..candidates {
        ui.press("j");
    }
    assert_eq!(ui.dialog_value("choice").as_deref(), Some("none"));
    assert_eq!(ui.dialog_value("can_confirm").as_deref(), Some("false"));
    ui.press("k");
    assert_eq!(
        ui.dialog_value("choice"),
        Some((candidates - 1).to_string())
    );
    ui.press("j enter");

    assert_eq!(
        ui.bills().dialog.as_deref(),
        Some("Pay"),
        "nothing confirmed"
    );
    assert_eq!(ui.bills().pay_mode.as_deref(), Some("Direct"));
    ui.press("left");
    assert_eq!(ui.bills().pay_mode.as_deref(), Some("Match"));
}

#[gpui::test]
fn typing_reaches_only_pay_it_directly_and_tab_moves_to_the_date(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    on_telstra(&mut ui);
    ui.press("p");
    let amount = ui.dialog_value("amount");

    ui.press("5 backspace");
    assert_eq!(ui.dialog_value("amount"), amount, "Match takes no typing");

    ui.press("right tab backspace");
    let date = ui.dialog_value("date").expect("the Date is drawn");
    ui.press("x");
    assert_eq!(ui.dialog_value("date"), Some(format!("{date}x")));
    assert_eq!(ui.dialog_value("amount"), amount);
    assert_eq!(ui.dialog_value("can_confirm").as_deref(), Some("false"));
    ui.press("enter");
    assert_eq!(
        ui.bills().dialog.as_deref(),
        Some("Pay"),
        "a bad date refuses"
    );
}

#[gpui::test]
fn escape_cancels_without_paying(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    on_bills(&mut ui);
    let before = ui.bills().rows;

    ui.press("p");
    ui.press("escape");

    let page = ui.bills();
    assert_eq!(page.dialog, None);
    assert_eq!(page.rows, before);
}

#[gpui::test]
fn a_failed_payment_puts_both_back_with_an_error_toast(app: &mut TestAppContext) {
    let mut ui = Harness::with_failing_payments(app);
    on_telstra(&mut ui);
    let stored = ui.bills().rows;

    ui.press("p right enter");
    assert_eq!(
        telstra_status(&mut ui),
        "Paid",
        "memory runs ahead of the file until the failure is heard"
    );

    ui.settle_writes();

    assert_eq!(ui.bills().rows, stored);
    let errors: Vec<String> = ui
        .toasts()
        .visible
        .into_iter()
        .filter(|toast| toast.kind == "Error")
        .map(|toast| toast.text)
        .collect();
    assert!(
        errors.contains(
            &"Couldn't save the change to transactions, so they are back as last saved".into()
        ),
        "{errors:?}"
    );
    assert!(
        errors
            .contains(&"Couldn't save the change to bills, so they are back as last saved".into()),
        "{errors:?}"
    );
}

#[gpui::test]
fn ctrl_enter_confirms_a_valid_pay_as_enter_does(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    on_telstra(&mut ui);
    ui.press("p");

    ui.press("ctrl-enter");

    assert_eq!(ui.bills().dialog, None);
    assert_eq!(telstra_status(&mut ui), "Paid");
}

#[gpui::test]
fn an_override_of_next_field_moves_pay_it_directly_to_the_date(app: &mut TestAppContext) {
    let mut ui = Harness::with_keybindings(app, common::overriding("next_field", "ctrl+n"));
    on_telstra(&mut ui);
    ui.press("p right");
    let mut amount = ui.dialog_value("amount").unwrap_or_default();
    let date = ui.dialog_value("date").unwrap_or_default();

    // `tab` is free, so the Amount keeps focus and the backspace edits it.
    ui.press("tab backspace");
    amount.pop();
    assert_eq!(ui.dialog_value("amount"), Some(amount.clone()));
    assert_eq!(ui.dialog_value("date"), Some(date.clone()));

    ui.press("ctrl-n x");
    assert_eq!(ui.dialog_value("date"), Some(format!("{date}x")));
    assert_eq!(ui.dialog_value("amount"), Some(amount));
}

/// Whether the chosen row's bounds sit inside the Dialog body's.
fn chosen_row_in_body(ui: &mut Harness<'_>) -> bool {
    let row = match ui.dialog_value("choice").as_deref() {
        Some("none") => "pay-bill-none-of-these".to_string(),
        Some(index) => format!("pay-bill-candidate-{index}"),
        None => return false,
    };
    let body = ui.bounds("pay-bill-body");
    let row = ui.bounds(&row);
    row.top() >= body.top() && row.bottom() <= body.bottom()
}

#[gpui::test]
fn j_and_k_keep_the_chosen_row_inside_the_body_on_a_small_window(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Large);
    on_telstra(&mut ui);
    ui.press("p");
    let candidates = ui.bills().pay_candidates.expect("the Pay Dialog is open");

    for _ in 0..candidates {
        ui.press("j");
        assert!(
            chosen_row_in_body(&mut ui),
            "{:?}",
            ui.dialog_value("choice")
        );
    }
    assert_eq!(ui.dialog_value("choice").as_deref(), Some("none"));
    for _ in 0..candidates {
        ui.press("k");
        assert!(
            chosen_row_in_body(&mut ui),
            "{:?}",
            ui.dialog_value("choice")
        );
    }
    assert_eq!(ui.dialog_value("choice").as_deref(), Some("0"));
}
