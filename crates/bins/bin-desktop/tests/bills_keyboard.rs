//! The Bills page driven by keys: tabs, row selection, the period and filter chips, and the plan,
//! pay and skip dialogs. State is read back from `Shell`.

mod common;

use bin_desktop::navigation::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;

fn on_bills(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g w");
    assert_eq!(ui.noun(), Noun::Bills);
    ui
}

#[gpui::test]
fn opens_on_the_schedule_for_the_current_month(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    let page = ui.bills();
    assert_eq!(page.tab, "Schedule");
    assert_eq!(page.period, (2026, 6));
    assert!(!page.all);
    assert_eq!(page.rows.len(), 7);
    assert_eq!(page.rows[0].plan, "Gym \u{2014} Fitness First");
    assert_eq!(page.selected, 0);
}

#[gpui::test]
fn tab_switches_between_schedule_and_planner(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    ui.press("tab");
    assert_eq!(ui.bills().tab, "Planner");
    ui.press("tab");
    assert_eq!(ui.bills().tab, "Schedule");
}

#[gpui::test]
fn j_k_g_and_capital_g_move_the_selection(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    ui.press("j j");
    assert_eq!(ui.bills().selected, 2);
    ui.press("k");
    assert_eq!(ui.bills().selected, 1);
    ui.press("shift-g");
    assert_eq!(ui.bills().selected, 6);
    ui.press("g g");
    assert_eq!(ui.bills().selected, 0);
}

#[gpui::test]
fn switching_tab_resets_the_selection(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    ui.press("j j");
    ui.press("tab");
    assert_eq!(ui.bills().selected, 0);
}

#[gpui::test]
fn brackets_step_the_period_and_zero_toggles_all(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    ui.press("]");
    assert_eq!(ui.bills().period, (2026, 7));
    ui.press("[ [");
    assert_eq!(ui.bills().period, (2026, 5));

    ui.press("0");
    assert!(ui.bills().all);
    assert!(ui.bills().rows.len() > 7);
    // A step from All returns to the month last viewed rather than moving it.
    ui.press("]");
    let page = ui.bills();
    assert!(!page.all);
    assert_eq!(page.period, (2026, 5));
}

#[gpui::test]
fn digit_keys_toggle_the_status_chips(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    // Chip 1 is Paid.
    ui.press("1");
    let page = ui.bills();
    assert!(!page.statuses_on.contains(&"Paid".to_string()));
    assert_eq!(page.rows.len(), 6);
    assert!(page.rows.iter().all(|row| row.status != "Paid"));

    ui.press("1");
    assert_eq!(ui.bills().rows.len(), 7);
}

#[gpui::test]
fn f_cycles_focus_through_the_filter_selects_and_off(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    ui.press("f");
    assert_eq!(ui.bills().filter_focus.as_deref(), Some("Plan"));
    ui.press("f");
    assert_eq!(ui.bills().filter_focus.as_deref(), Some("Category"));
    ui.press("f");
    assert_eq!(ui.bills().filter_focus.as_deref(), Some("Account"));
    ui.press("f");
    assert_eq!(ui.bills().filter_focus, None);
}

#[gpui::test]
fn pay_opens_the_dialog_and_enter_marks_the_row_paid(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    ui.press("p");
    let page = ui.bills();
    assert_eq!(page.dialog.as_deref(), Some("Pay"));
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Dialog);
    assert!(page.pay_mode.is_some());

    // Leave Match for the Pay panel, whose amount and date default from the Plan.
    ui.press("right");
    assert_eq!(ui.bills().pay_mode.as_deref(), Some("Direct"));
    ui.press("enter");

    let page = ui.bills();
    assert_eq!(page.dialog, None);
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Normal);
    assert_eq!(
        page.rows.iter().filter(|row| row.status == "Paid").count(),
        2
    );
}

#[gpui::test]
fn escape_cancels_the_pay_dialog_without_paying(app: &mut TestAppContext) {
    let mut ui = on_bills(app);
    let before = ui.bills().rows;

    ui.press("p");
    ui.press("escape");

    let page = ui.bills();
    assert_eq!(page.dialog, None);
    assert_eq!(page.rows, before);
}

#[gpui::test]
fn skip_confirms_with_enter(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    ui.press("s");
    assert_eq!(ui.bills().dialog.as_deref(), Some("Skip"));
    ui.press("enter");

    let page = ui.bills();
    assert_eq!(page.dialog, None);
    assert_eq!(
        page.rows
            .iter()
            .filter(|row| row.status == "Skipped")
            .count(),
        2
    );
}

#[gpui::test]
fn escape_cancels_the_skip_dialog(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    ui.press("s escape");

    let page = ui.bills();
    assert_eq!(page.dialog, None);
    assert_eq!(
        page.rows
            .iter()
            .filter(|row| row.status == "Skipped")
            .count(),
        1
    );
}

#[gpui::test]
fn pay_and_skip_refuse_a_row_with_nothing_to_settle(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    // The last row is the Paid one.
    ui.press("shift-g");
    ui.press("p");
    assert_eq!(ui.bills().dialog, None);
    assert!(ui.read(|s| s.status_message().is_some()));

    ui.press("s");
    assert_eq!(ui.bills().dialog, None);
}

#[gpui::test]
fn n_opens_the_add_dialog_and_escape_closes_it(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    ui.press("n");
    assert_eq!(ui.bills().dialog.as_deref(), Some("Add"));
    ui.press("g y m");
    assert_eq!(ui.bills().plan_name.as_deref(), Some("gym"));
    ui.press("escape");

    let page = ui.bills();
    assert_eq!(page.dialog, None);
    assert_eq!(page.plans.len(), 10);
}

#[gpui::test]
fn the_add_dialog_saves_a_new_plan(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    ui.press("n");
    ui.press("t o l l");
    // Name, Category, Unit, Account, Payee, Amount: tab to the Amount field.
    ui.press("tab tab tab tab tab");
    ui.press("4 5");
    ui.press("enter");

    let page = ui.bills();
    assert_eq!(page.dialog, None);
    assert_eq!(page.plans.len(), 11);
    assert!(page.plans.contains(&"toll".to_string()));
}

#[gpui::test]
fn e_on_the_planner_edits_the_selected_plan(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    ui.press("tab e");

    let page = ui.bills();
    assert_eq!(page.dialog.as_deref(), Some("Edit"));
    assert_eq!(
        page.plan_name.as_deref(),
        Some("Car Insurance \u{2014} AAMI")
    );
    ui.press("escape");
    assert_eq!(ui.bills().dialog, None);
}

#[gpui::test]
fn enter_on_a_planner_row_edits_it(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    ui.press("tab j enter");

    assert_eq!(ui.bills().dialog.as_deref(), Some("Edit"));
    assert_eq!(ui.bills().plan_name.as_deref(), Some("Car Wash Membership"));
}
