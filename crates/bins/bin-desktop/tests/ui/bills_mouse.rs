//! The Bills page driven by clicks: tabs, period nav, status chips, rows and their Pay and Skip
//! buttons, the add button, and the plan, pay and skip dialogs' buttons. Only elements tagged with
//! `debug_selector` are clicked; state is read back from `Shell`.

use super::common;
use lib_core::BillPlanId;

use bin_desktop::navigation::nav::Noun;
use common::Harness;
use gpui::TestAppContext;

fn on_bills(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g w");
    assert_eq!(ui.noun(), Noun::Bills);
    ui
}

#[gpui::test]
fn clicking_the_tabs_switches_them(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    ui.click("bills-tab-Planner");
    assert_eq!(ui.bills().tab, "Planner");
    ui.click("bills-tab-Schedule");
    assert_eq!(ui.bills().tab, "Schedule");
}

#[gpui::test]
fn clicking_a_row_selects_it(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    ui.click("bills-row-2");
    assert_eq!(ui.bills().selected, 2);
    ui.click("bills-row-0");
    assert_eq!(ui.bills().selected, 0);
}

#[gpui::test]
fn the_period_arrows_and_all_button_change_the_schedule(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    ui.click("bills-period-next");
    assert_eq!(ui.bills().period, (2026, 7));
    ui.click("bills-period-prev");
    ui.click("bills-period-prev");
    assert_eq!(ui.bills().period, (2026, 5));

    ui.click("bills-period-all");
    assert!(ui.bills().all);
    ui.click("bills-period-all");
    assert!(!ui.bills().all);
}

#[gpui::test]
fn clicking_a_status_chip_toggles_it(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    ui.click("bills-filter-chip-0");
    let page = ui.bills();
    assert!(!page.statuses_on.contains(&"Paid".to_string()));
    assert!(page.rows.iter().all(|row| row.status != "Paid"));

    ui.click("bills-filter-chip-0");
    assert_eq!(ui.bills().rows.len(), 7);
}

#[gpui::test]
fn the_add_button_opens_the_dialog_and_cancel_closes_it(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    ui.click("bills-add");
    assert_eq!(ui.bills().dialog.as_deref(), Some("Add"));
    ui.click("add-bill-plan-cancel");
    assert_eq!(ui.bills().dialog, None);
}

// The Pay dialog is taller than the 1080px test window, so its mode switch and buttons have no
// bounds to click; the keyboard tests cover confirm and cancel, and a click here stops at opening.
#[gpui::test]
fn a_row_pay_button_selects_the_row_and_opens_the_pay_dialog(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    ui.click("bills-pay-1");

    let page = ui.bills();
    assert_eq!(page.dialog.as_deref(), Some("Pay"));
    assert_eq!(page.selected, 1);
    ui.press("escape");
    assert_eq!(ui.bills().dialog, None);
}

#[gpui::test]
fn a_row_skip_button_opens_the_dialog_and_confirm_skips_it(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    ui.click("bills-skip-0");
    assert_eq!(ui.bills().dialog.as_deref(), Some("Skip"));
    ui.click("skip-bill-confirm");

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
fn the_skip_dialog_cancel_button_closes_it(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    ui.click("bills-skip-0");
    ui.click("skip-bill-cancel");

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
fn a_planner_edit_button_opens_the_edit_dialog(app: &mut TestAppContext) {
    let mut ui = on_bills(app);
    ui.click("bills-tab-Planner");

    let first = ui.bills().plans.len();
    assert!(first > 0);
    ui.click(&format!("bills-plan-row-{}", BillPlanId::seed(1)));
    ui.click(&format!("bills-plan-edit-{}", BillPlanId::seed(1)));

    assert_eq!(ui.bills().dialog.as_deref(), Some("Edit"));
}

#[gpui::test]
fn a_filter_select_opens_on_click_and_an_option_scopes_the_schedule(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    ui.click("bills-filter-plan");
    assert_eq!(ui.bills().filter_focus.as_deref(), Some("Plan"));
    // Option 0 is "All bills"; option 1 is the first Bill Plan in Planner order.
    ui.click("bills-filter-plan-option-1");
    let page = ui.bills();
    assert!(!page.rows.is_empty(), "the first Plan has a row this month");
    assert!(page.rows.iter().all(|row| row.plan == page.plans[0]));
}

#[gpui::test]
fn a_paid_row_view_button_opens_its_transaction(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    // The last row is the Paid one.
    ui.click("bills-view-6");
    assert_eq!(ui.noun(), Noun::Transactions);
}
