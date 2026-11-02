//! The Budgets page driven by clicks: tabs, period nav, rows and their edit action, Plan cells and
//! the dialogs' buttons. Only elements tagged with `debug_selector` are clicked; state is read back
//! from `Root`.

use super::common;

use bin_desktop::navigation::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;
use lib_budgets::PERSONAL_SPENDING_ID;
use lib_core::BudgetId;

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

#[gpui::test]
fn clicking_the_title_opens_the_switcher(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.click("budgets-title");
    assert_eq!(ui.budgets().dialog.as_deref(), Some("Switcher"));
}

#[gpui::test]
fn the_primary_action_follows_the_tab(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    // Progress: Edit plan.
    ui.click("budgets-primary-action");
    assert_eq!(ui.budgets().tab, "Plan");
    // Plan: + Budget a category.
    ui.click("budgets-primary-action");
    assert_eq!(ui.budgets().dialog.as_deref(), Some("EditLimit"));
}

#[gpui::test]
fn the_fill_button_opens_fill(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.click("budgets-tab-Plan");
    ui.click("budgets-fill");
    assert_eq!(ui.budgets().dialog.as_deref(), Some("Fill"));
}

#[gpui::test]
fn clicking_a_history_cell_moves_there_and_a_second_click_opens_its_detail(
    app: &mut TestAppContext,
) {
    let mut ui = on_budgets(app);

    ui.click("budgets-tab-History");
    ui.click("budgets-history-1-0");
    assert_eq!(ui.budgets().dialog, None);
    ui.click("budgets-history-1-0");
    assert_eq!(ui.budgets().dialog.as_deref(), Some("Detail"));
}

#[gpui::test]
fn a_dialog_opened_mid_edit_abandons_the_plan_cell(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.click("budgets-tab-Plan");
    ui.click("budgets-plan-1-2");
    assert!(ui.budgets().plan_edit.is_some());
    ui.click("budgets-title");
    let page = ui.budgets();
    assert_eq!(page.dialog.as_deref(), Some("Switcher"));
    assert_eq!(page.plan_edit, None);
    ui.press("escape");
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Normal);
    // Back in Normal, keys move the grid again rather than typing into a stale cell.
    ui.press("l");
    assert_eq!(ui.budgets().plan_cursor, (1, 3));
}

#[gpui::test]
fn export_csv_on_history_writes_the_file_chosen(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);
    let folder = tempfile::tempdir().expect("a temp folder");
    let path = folder.path().join("history.csv");
    ui.click("budgets-tab-History");

    ui.click("budgets-primary-action");
    ui.answer_save_dialog(Some(path.clone()));

    let csv = std::fs::read_to_string(&path).expect("the export wrote the file");
    assert!(csv.starts_with("Category,"), "a header row: {csv}");
    let shown = path.display().to_string();
    assert!(
        ui.toasts()
            .visible
            .into_iter()
            .any(|toast| { toast.kind == "Success" && toast.text.contains(&shown) })
    );
}

#[gpui::test]
fn the_details_buttons_hand_off_to_transactions_and_to_edit_budget(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.click("budgets-progress-row-1");
    ui.click("budgets-progress-row-1");
    assert_eq!(ui.budgets().dialog.as_deref(), Some("Detail"));
    ui.click("budgets-detail-edit");
    assert_eq!(ui.budgets().dialog.as_deref(), Some("EditLimit"));
    ui.click("budgets-limit-cancel");

    ui.click("budgets-progress-row-1");
    assert_eq!(ui.budgets().dialog.as_deref(), Some("Detail"));
    let before = ui.commits();
    ui.click("budgets-detail-transactions");
    assert_eq!(ui.noun(), Noun::Transactions);
    assert_eq!(ui.budgets().dialog, None);
    assert_eq!(ui.commits(), before + 1, "the click is one input");
}

#[gpui::test]
fn the_limit_dialog_segments_and_confirm_save_the_amount(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);
    ui.click("budgets-progress-action-1");
    assert_eq!(ui.budgets().dialog.as_deref(), Some("EditLimit"));

    // Month only, then Carry unspent; the preview and the field stay on the form.
    ui.click("budgets-limit-span-0");
    ui.click("budgets-limit-rollover-1");
    assert_eq!(ui.budgets().dialog.as_deref(), Some("EditLimit"));
    ui.click("budgets-limit-amount");
    assert_eq!(ui.dialog_value("can_confirm").as_deref(), Some("true"));

    let before = ui.commits();
    ui.click("budgets-limit-confirm");

    assert_eq!(ui.budgets().dialog, None);
    assert_eq!(ui.commits(), before + 1, "the click is one input");
    assert!(
        ui.toast_texts()
            .iter()
            .any(|text| text.starts_with("Saved the budget for")),
        "{:?}",
        ui.toast_texts()
    );
}

#[gpui::test]
fn the_limit_dialogs_starting_select_opens_and_a_click_on_a_month_chooses_it(
    app: &mut TestAppContext,
) {
    let mut ui = on_budgets(app);
    ui.click("budgets-progress-action-1");

    ui.click("budgets-limit-starting");
    assert_eq!(ui.budgets().dialog.as_deref(), Some("EditLimit"));
    ui.click("budgets-limit-starting-option-1");
    assert_eq!(ui.budgets().dialog.as_deref(), Some("EditLimit"));
    ui.click("budgets-limit-cancel");
    assert_eq!(ui.budgets().dialog, None);
}

#[gpui::test]
fn the_stop_dialog_confirm_stops_the_category(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);
    ui.click("budgets-progress-action-1");
    ui.click("budgets-limit-stop");
    assert_eq!(ui.budgets().dialog.as_deref(), Some("Stop"));

    let before = ui.commits();
    ui.click("budgets-stop-confirm");

    assert_eq!(ui.budgets().dialog, None);
    assert_eq!(ui.commits(), before + 1, "the click is one input");
    assert!(
        ui.toast_texts()
            .iter()
            .any(|text| text.starts_with("Stopped budgeting")),
        "{:?}",
        ui.toast_texts()
    );
}

#[gpui::test]
fn clicking_a_fill_source_chooses_it_and_cancel_closes_the_dialog(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);
    ui.click("budgets-tab-Plan");
    ui.click("budgets-fill");
    assert_eq!(ui.budgets().fill_source.as_deref(), Some("PreviousMonth"));

    ui.click("budgets-fill-Average");
    assert_eq!(ui.budgets().fill_source.as_deref(), Some("Average"));
    ui.click("budgets-fill-PreviousMonth");
    assert_eq!(ui.budgets().fill_source.as_deref(), Some("PreviousMonth"));
    ui.click("budgets-fill-cancel");
    assert_eq!(ui.budgets().dialog, None);
}

#[gpui::test]
fn the_new_budget_dialog_start_cells_and_cancel(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);
    ui.click("budgets-title");
    ui.click("budgets-switcher-new");
    assert_eq!(ui.budgets().dialog.as_deref(), Some("Budget"));

    ui.click("budgets-new-start-2");
    ui.click("budgets-new-start-0");
    ui.click("budgets-new-name");
    assert_eq!(ui.budgets().dialog.as_deref(), Some("Budget"));
    assert_eq!(
        ui.dialog_value("can_confirm").as_deref(),
        Some("false"),
        "a Budget needs a name"
    );
    ui.click("budgets-new-confirm");
    assert_eq!(
        ui.budgets().dialog.as_deref(),
        Some("Budget"),
        "an incomplete form takes no confirm"
    );
    ui.click("budgets-new-cancel");
    assert_eq!(ui.budgets().dialog, None);
}

fn open_manage(ui: &mut Harness<'_>) {
    ui.click("budgets-title");
    ui.click("budgets-switcher-manage");
    assert_eq!(ui.budgets().dialog.as_deref(), Some("Manage"));
}

fn manage_rows(ui: &mut Harness<'_>) -> Vec<(String, bool, bool)> {
    ui.budgets().manage_rows.unwrap_or_default()
}

/// The selector of one of a Budget row's action links.
fn link(id: BudgetId, action: &str) -> String {
    format!("budgets-manage-{id}-{action}")
}

#[gpui::test]
fn the_switchers_manage_button_opens_manage_budgets_and_close_shuts_it(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    open_manage(&mut ui);
    assert_eq!(manage_rows(&mut ui).len(), 2);
    ui.click("budgets-manage-close");
    assert_eq!(ui.budgets().dialog, None);
}

#[gpui::test]
fn clicking_a_row_moves_the_manage_cursor(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);
    open_manage(&mut ui);

    ui.click("budgets-manage-row-1");
    assert_eq!(ui.budgets().manage_selected, Some(1));
    ui.click("budgets-manage-row-0");
    assert_eq!(ui.budgets().manage_selected, Some(0));
}

#[gpui::test]
fn the_archive_and_restore_links_archive_and_restore_a_budget(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);
    open_manage(&mut ui);
    let joint = BudgetId::seed(2);

    ui.click(&link(joint, "Archive"));
    assert_eq!(
        manage_rows(&mut ui)[1],
        ("Joint household".to_string(), false, true)
    );
    assert!(ui.is_drawn(&link(joint, "Restore")));
    ui.click(&link(joint, "Restore"));
    assert_eq!(
        manage_rows(&mut ui)[1],
        ("Joint household".to_string(), false, false)
    );
    assert_eq!(
        ui.budgets().manage_selected,
        Some(1),
        "the click's row stays under the cursor"
    );
}

#[gpui::test]
fn the_set_default_link_moves_the_default_and_the_default_row_offers_no_archive_link(
    app: &mut TestAppContext,
) {
    let mut ui = on_budgets(app);
    open_manage(&mut ui);
    let personal = PERSONAL_SPENDING_ID;
    let joint = BudgetId::seed(2);
    assert!(
        !ui.is_drawn(&link(personal, "Archive")),
        "the default can't be archived, so the link isn't offered"
    );

    ui.click(&link(joint, "SetDefault"));
    assert_eq!(
        manage_rows(&mut ui),
        [
            ("Personal spending".to_string(), false, false),
            ("Joint household".to_string(), true, false),
        ]
    );
    assert!(
        ui.is_drawn(&link(personal, "Archive")),
        "the old default can now be archived"
    );
}
