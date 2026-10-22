//! The Pay Dialog driven by clicks, on the in-memory ports: its mode switch, Match rows, fields and
//! buttons each reach the `lib-bill-payments` Presenter, and what it changes is drawn. The Dialog
//! stays inside the 1080px test window, its body scrolling, so every button has bounds to click.
//! Only elements tagged with `debug_selector` are clicked.

#![expect(
    clippy::expect_used,
    reason = "test support: a Telstra Internet row the stub doesn't seed should fail the test loudly"
)]

use super::common;

use bin_desktop::navigation::nav::Noun;
use common::Harness;
use gpui::{TestAppContext, point, px};

/// The status of the Telstra Internet row, wherever the Schedule's order has put it.
fn telstra_status(ui: &mut Harness<'_>) -> String {
    ui.bills()
        .rows
        .into_iter()
        .find(|row| row.plan == "Telstra Internet")
        .expect("Telstra Internet is due this month")
        .status
}

/// The Bills page with the Pay Dialog opened by a click on the Telstra Internet row's Pay button,
/// which has Match candidates.
fn paying_telstra(ui: &mut Harness<'_>) {
    ui.press("g w");
    assert_eq!(ui.noun(), Noun::Bills);
    let row = ui
        .bills()
        .rows
        .iter()
        .position(|row| row.plan == "Telstra Internet")
        .expect("Telstra Internet is due this month");
    ui.click(&format!("bills-pay-{row}"));
    assert_eq!(ui.bills().dialog.as_deref(), Some("Pay"));
}

#[gpui::test]
fn the_switch_and_buttons_are_inside_the_window(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    paying_telstra(&mut ui);

    for selector in [
        "pay-bill-mode-0",
        "pay-bill-mode-1",
        "pay-bill-confirm",
        "pay-bill-cancel",
    ] {
        let bounds = ui.bounds(selector);
        assert!(
            bounds.top() >= px(0.0) && bounds.bottom() <= px(1080.0),
            "{selector} is off screen: {bounds:?}"
        );
    }
}

#[gpui::test]
fn the_mode_switch_changes_panel(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    paying_telstra(&mut ui);
    assert_eq!(ui.bills().pay_mode.as_deref(), Some("Match"));

    ui.click("pay-bill-mode-0");
    assert_eq!(ui.bills().pay_mode.as_deref(), Some("Direct"));
    assert!(ui.is_drawn("pay-bill-amount"));
    assert!(ui.is_drawn("pay-bill-date"));
    ui.click("pay-bill-mode-1");
    assert_eq!(ui.bills().pay_mode.as_deref(), Some("Match"));
    assert!(ui.is_drawn("pay-bill-candidate-0"));
}

#[gpui::test]
fn clicking_a_candidate_chooses_it_and_confirm_matches_it(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    paying_telstra(&mut ui);

    // Eighteen candidates put "None of these" below the body's fold: scroll it into view.
    let body = ui.bounds("pay-bill-candidate-0").center();
    ui.scroll_at(body, point(px(0.0), px(-2000.0)));
    ui.click("pay-bill-none-of-these");
    assert_eq!(ui.bills().pay_mode.as_deref(), Some("Direct"));
    ui.click("pay-bill-mode-1");
    ui.click("pay-bill-candidate-0");
    assert_eq!(ui.dialog_value("choice").as_deref(), Some("0"));
    ui.click("pay-bill-confirm");

    assert_eq!(ui.bills().dialog, None);
    assert_eq!(telstra_status(&mut ui), "Paid");
}

#[gpui::test]
fn clicking_a_field_focuses_it_for_typing_and_confirm_pays(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    paying_telstra(&mut ui);
    ui.click("pay-bill-mode-0");
    let amount = ui.dialog_value("amount").expect("the Amount is drawn");

    ui.click("pay-bill-date");
    for _ in 0..12 {
        ui.press("backspace");
    }
    ui.press("t o d a y");
    assert_eq!(ui.dialog_value("date").as_deref(), Some("today"));
    assert_eq!(ui.dialog_value("amount"), Some(amount.clone()));
    ui.click("pay-bill-amount");
    ui.press("1");
    assert_eq!(ui.dialog_value("amount"), Some(format!("{amount}1")));
    ui.click("pay-bill-confirm");

    assert_eq!(ui.bills().dialog, None);
    assert_eq!(telstra_status(&mut ui), "Paid");
}

#[gpui::test]
fn cancel_closes_without_paying(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    paying_telstra(&mut ui);
    let before = telstra_status(&mut ui);

    ui.click("pay-bill-cancel");

    assert_eq!(ui.bills().dialog, None);
    assert_eq!(telstra_status(&mut ui), before);
}
