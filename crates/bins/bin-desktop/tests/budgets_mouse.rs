//! The Budgets page driven by clicks: tabs, period nav, rows and their edit action, Plan cells and
//! the dialogs' buttons. Only elements tagged with `debug_selector` are clicked; state is read back
//! from `Shell`.

mod common;

use bin_desktop::navigation::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;

fn on_budgets(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g b");
    assert_eq!(ui.noun(), Noun::Budgets);
    ui
}

#[gpui::test]
fn clicking_the_tabs_switches_them(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.click("budgets-tab-Plan");
    assert_eq!(ui.budgets().tab, "Plan");
    ui.click("budgets-tab-History");
    assert_eq!(ui.budgets().tab, "History");
    ui.click("budgets-tab-Progress");
    assert_eq!(ui.budgets().tab, "Progress");
}

#[gpui::test]
fn the_period_arrows_step_the_month(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.click("budgets-period-next");
    assert_eq!(ui.budgets().period, (2026, 7));
    ui.click("budgets-period-prev");
    ui.click("budgets-period-prev");
    assert_eq!(ui.budgets().period, (2026, 5));
}

#[gpui::test]
fn the_range_arrows_step_the_plan_and_history_ranges(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.click("budgets-tab-Plan");
    ui.click("budgets-period-next");
    assert_eq!(ui.budgets().plan_start, (2026, 5));
    ui.click("budgets-tab-History");
    ui.click("budgets-period-prev");
    assert_eq!(ui.budgets().history_end, (2026, 5));
}

#[gpui::test]
fn clicking_a_row_selects_it_and_a_second_click_opens_its_detail(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.click("budgets-progress-row-1");
    let page = ui.budgets();
    assert_eq!(page.selected, 1);
    assert_eq!(page.dialog, None);
    ui.click("budgets-progress-row-1");
    assert_eq!(ui.budgets().dialog.as_deref(), Some("Detail"));
    ui.click("budgets-detail-close");
    assert_eq!(ui.budgets().dialog, None);
}

#[gpui::test]
fn a_rows_edit_action_opens_the_limit_dialog_and_cancel_closes_it(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.click("budgets-progress-action-1");
    let page = ui.budgets();
    assert_eq!(page.selected, 1);
    assert_eq!(page.dialog.as_deref(), Some("EditLimit"));
    ui.click("budgets-limit-cancel");
    assert_eq!(ui.budgets().dialog, None);
}

#[gpui::test]
fn the_limit_dialogs_stop_link_opens_stop_budgeting(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.click("budgets-progress-action-1");
    ui.click("budgets-limit-stop");
    assert_eq!(ui.budgets().dialog.as_deref(), Some("Stop"));
    ui.click("budgets-stop-cancel");
    assert_eq!(ui.budgets().dialog, None);
}

#[gpui::test]
fn clicking_a_plan_cell_in_an_open_month_starts_an_edit(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.click("budgets-tab-Plan");
    ui.click("budgets-plan-1-2");
    let page = ui.budgets();
    assert_eq!(page.plan_cursor, (1, 2));
    assert!(page.plan_edit.is_some());
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Insert);
}

#[gpui::test]
fn clicking_a_closed_plan_cell_only_moves_the_cursor(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.click("budgets-tab-Plan");
    ui.click("budgets-plan-1-0");
    let page = ui.budgets();
    assert_eq!(page.plan_cursor, (1, 0));
    assert_eq!(page.plan_edit, None);
}

#[gpui::test]
fn the_known_costs_link_opens_the_bills_schedule_on_the_period_shown(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.click("budgets-period-prev");
    let shown = ui.budgets().period;
    ui.click("budgets-known-schedule");

    assert_eq!(ui.noun(), Noun::Bills);
    let bills = ui.bills();
    assert_eq!(bills.tab, "Schedule");
    assert_eq!(bills.period, shown);
}
