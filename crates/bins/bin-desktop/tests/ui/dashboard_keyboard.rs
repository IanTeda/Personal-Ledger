//! The Dashboard driven by keys: it is the start page, lists the Bills needing attention, scrolls
//! on movement keys and takes `/` into a Search with nothing to type into. State is read back from
//! `Root`.

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
    assert!(
        !bills.is_empty(),
        "the sample ledger has Bills to attend to"
    );
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

/// The Plans of the entries due in `month` (`2026-08`) that Needs Attention shows, by the due
/// date in each row's selector.
fn attention_due_in(ui: &mut Harness<'_>, month: &str) -> Vec<String> {
    ui.dashboard_bills()
        .into_iter()
        .filter(|bill| {
            let due = &bill.selector[bill.selector.len() - "2026-08-31".len()..];
            due.starts_with(month)
        })
        .map(|bill| bill.plan)
        .collect()
}

/// The one August entry the sample stores before August is generated: a long Recurrence keeps one
/// future entry past the horizon.
const STORED_IN_AUGUST: &str = "Council Rates";

#[gpui::test]
fn a_new_month_generates_its_bills_while_the_dashboard_shows(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    assert!(
        attention_due_in(&mut ui, "2026-08").is_empty(),
        "nothing in August needs attention in June"
    );

    ui.turn_date_to(common::date(2026, 8, 31));

    assert_eq!(ui.noun(), Noun::Dashboard);
    let august = attention_due_in(&mut ui, "2026-08");
    assert!(
        august.iter().any(|plan| plan != STORED_IN_AUGUST),
        "August's entries are generated and due: {august:?}"
    );
}

#[gpui::test]
fn a_day_tick_commits_once_and_a_month_turn_lands_its_new_bills_in_that_commit(
    app: &mut TestAppContext,
) {
    let mut ui = Harness::new(app);

    let before = ui.commits();
    ui.turn_date_to(common::date(2026, 6, 30));
    assert_eq!(
        ui.commits(),
        before + 1,
        "a tick is one input, written or not"
    );

    let before = ui.commits();
    ui.turn_date_to(common::date(2026, 8, 31));
    assert_eq!(
        ui.commits(),
        before + 1,
        "the month's generated Bills land in the tick's one commit"
    );
    assert!(
        attention_due_in(&mut ui, "2026-08")
            .iter()
            .any(|plan| plan != STORED_IN_AUGUST),
        "the month turned and generated its entries"
    );
}

#[gpui::test]
fn a_new_months_generation_that_fails_to_save_rolls_back_with_an_error_toast(
    app: &mut TestAppContext,
) {
    let mut ui = Harness::with_failing_bills(app);

    ui.turn_date_to(common::date(2026, 8, 31));
    ui.settle_writes();

    assert_eq!(
        attention_due_in(&mut ui, "2026-08"),
        vec![STORED_IN_AUGUST.to_string()],
        "only what was stored before the month turned"
    );
    assert!(
        ui.toasts()
            .visible
            .iter()
            .any(|toast| toast.kind == "Error"),
        "the failed save raises an error Toast"
    );
}

/// Presses `confirm` and says whether the Dashboard asked to redraw for it, though another page
/// shows: the write's module announces its domain, with no Ledger read copy in between (#1028).
fn redraws_on(ui: &mut Harness<'_>, confirm: &str) -> bool {
    let redraws = ui.count_dashboard_redraws();
    ui.press(confirm);
    redraws.get() > 0
}

#[gpui::test]
fn a_skipped_bill_redraws_the_dashboard(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("g w s");
    assert_eq!(ui.bills().dialog.as_deref(), Some("Skip"));

    assert!(redraws_on(&mut ui, "enter"), "a Bills write");
    assert_eq!(ui.bills().dialog, None, "the Skip landed");
}

#[gpui::test]
fn a_saved_budget_limit_redraws_the_dashboard(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("g b j e");
    assert_eq!(ui.budgets().dialog.as_deref(), Some("EditLimit"));
    let prefilled = ui.budgets().limit_amount.unwrap_or_default().len();
    for _ in 0..prefilled {
        ui.press("backspace");
    }
    ui.press("7");

    assert!(redraws_on(&mut ui, "enter"), "a Budgets write");
    assert_eq!(ui.budgets().dialog, None, "the Limit saved");
}

/// The Import writes only Transactions and Payees, and the Dashboard reads no Payees: its
/// redraw is the Transactions' announcement.
#[gpui::test]
fn an_import_redraws_the_dashboard(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    let redraws = ui.count_dashboard_redraws();

    super::import_keyboard::commit_the_candle_shop_as_a_new_payee(&mut ui);

    assert!(redraws.get() > 0, "a Transactions write");
}
