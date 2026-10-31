//! The Budgets page driven by keys: tabs, the period and range navs, row selection, the Plan grid
//! and its cell editor, and the detail, limit, stop, fill, switcher and new-budget dialogs. State
//! is read back from `Root`.

use super::common;

use bin_desktop::navigation::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;

fn mode(ui: &mut Harness<'_>) -> InputMode {
    ui.read(|shell| shell.nav().mode())
}

fn on_budgets(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g b");
    assert_eq!(ui.noun(), Noun::Budgets);
    ui
}

#[gpui::test]
fn opens_on_progress_for_the_current_month(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    let page = ui.budgets();
    assert_eq!(page.tab, "Progress");
    assert_eq!(page.budget, "Personal spending");
    assert_eq!(page.period, (2026, 6));
    assert_eq!(page.rows.len(), 10);
    assert_eq!(page.rows[0].category, "Housing");
    assert!(page.rows[0].is_parent);
    assert_eq!(page.selected, 0);
    assert_eq!(page.dialog, None);
}

#[gpui::test]
fn digits_pick_the_tab(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.press("2");
    assert_eq!(ui.budgets().tab, "Plan");
    ui.press("3");
    assert_eq!(ui.budgets().tab, "History");
    ui.press("1");
    assert_eq!(ui.budgets().tab, "Progress");
}

#[gpui::test]
fn brackets_step_the_period_and_reset_the_selection(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.press("j j");
    ui.press("]");
    let page = ui.budgets();
    assert_eq!(page.period, (2026, 7));
    assert_eq!(page.selected, 0);
    ui.press("[ [");
    assert_eq!(ui.budgets().period, (2026, 5));
}

#[gpui::test]
fn j_k_g_and_capital_g_move_the_selection(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.press("j j");
    assert_eq!(ui.budgets().selected, 2);
    ui.press("k");
    assert_eq!(ui.budgets().selected, 1);
    ui.press("shift-g");
    assert_eq!(ui.budgets().selected, 9);
    ui.press("g g");
    assert_eq!(ui.budgets().selected, 0);
}

#[gpui::test]
fn enter_opens_the_category_detail_and_escape_closes_it(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.press("j enter");
    assert_eq!(ui.budgets().dialog.as_deref(), Some("Detail"));
    assert_eq!(mode(&mut ui), InputMode::Dialog);
    ui.press("escape");
    assert_eq!(ui.budgets().dialog, None);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn the_detail_transactions_key_shows_the_category_s_transactions_in_one_commit(
    app: &mut TestAppContext,
) {
    let mut ui = on_budgets(app);
    ui.press("j enter");
    assert_eq!(ui.budgets().dialog.as_deref(), Some("Detail"));

    let before = ui.commits();
    ui.press("t");

    assert_eq!(ui.noun(), Noun::Transactions);
    assert_eq!(ui.budgets().dialog, None, "the hand-off closes the detail");
    let page = ui.transactions();
    assert_eq!(page.selected, 0);
    assert!(page.filters_changed, "the list is filtered to the Category");
    assert!(
        page.chips
            .iter()
            .any(|chip| chip.active && chip.label.starts_with("category")),
        "the Category chip is active: {:?}",
        page.chips
    );
    assert_eq!(ui.commits(), before + 1, "the key is one input");
}

#[gpui::test]
fn the_detail_edit_key_opens_the_limit_dialog(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.press("j enter e");
    let page = ui.budgets();
    assert_eq!(page.dialog.as_deref(), Some("EditLimit"));
    assert!(page.limit_amount.is_some());
}

#[gpui::test]
fn e_edits_a_leaf_row_but_not_a_parents_rollup(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.press("e");
    assert_eq!(ui.budgets().dialog, None);
    ui.press("j e");
    assert_eq!(ui.budgets().dialog.as_deref(), Some("EditLimit"));
    ui.press("escape");
    assert_eq!(ui.budgets().dialog, None);
}

#[gpui::test]
fn s_opens_stop_budgeting_for_the_leaf_under_the_cursor(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.press("j s");
    let page = ui.budgets();
    assert_eq!(page.dialog.as_deref(), Some("Stop"));
    assert!(page.stop_category.is_some());
    ui.press("escape");
    assert_eq!(ui.budgets().dialog, None);
}

#[gpui::test]
fn capital_b_opens_the_switcher_listing_the_budgets(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.press("shift-b");
    let page = ui.budgets();
    assert_eq!(page.dialog.as_deref(), Some("Switcher"));
    let names = page.switcher_names.expect("the switcher is open");
    assert!(names.contains(&"Personal spending".to_string()));
    ui.press("escape");
    assert_eq!(ui.budgets().dialog, None);
}

#[gpui::test]
fn n_opens_the_new_budget_dialog_and_typing_fills_its_name(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.press("n");
    let page = ui.budgets();
    assert_eq!(page.dialog.as_deref(), Some("Budget"));
    assert_eq!(page.budget_name.as_deref(), Some(""));
    ui.press("t r i p");
    assert_eq!(ui.budgets().budget_name.as_deref(), Some("trip"));
    ui.press("escape");
    assert_eq!(ui.budgets().dialog, None);
}

#[gpui::test]
fn plan_keys_move_the_cursor_and_step_the_range(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.press("2");
    assert_eq!(ui.budgets().plan_start, (2026, 4));
    ui.press("l l j");
    assert_eq!(ui.budgets().plan_cursor, (1, 2));
    ui.press("h");
    assert_eq!(ui.budgets().plan_cursor, (1, 1));
    ui.press("]");
    assert_eq!(ui.budgets().plan_start, (2026, 5));
    ui.press("[ [");
    assert_eq!(ui.budgets().plan_start, (2026, 3));
}

#[gpui::test]
fn plan_i_edits_an_open_month_and_escape_abandons_it(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    // The range opens two months back, so column 2 is the current month.
    ui.press("2 j l l i");
    assert_eq!(mode(&mut ui), InputMode::Insert);
    ui.press("9 0 0");
    assert_eq!(ui.budgets().plan_edit.as_deref(), Some("900"));
    ui.press("escape");
    let page = ui.budgets();
    assert_eq!(page.plan_edit, None);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn plan_f_opens_fill_and_escape_closes_it(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.press("2 f");
    let page = ui.budgets();
    assert_eq!(page.dialog.as_deref(), Some("Fill"));
    assert_eq!(page.fill_source.as_deref(), Some("PreviousMonth"));
    ui.press("escape");
    assert_eq!(ui.budgets().dialog, None);
}

#[gpui::test]
fn history_brackets_step_the_range(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.press("3");
    assert_eq!(ui.budgets().history_end, (2026, 6));
    ui.press("[");
    assert_eq!(ui.budgets().history_end, (2026, 5));
    ui.press("]");
    assert_eq!(ui.budgets().history_end, (2026, 6));
}

#[gpui::test]
fn plan_enter_commits_the_edit_and_returns_to_normal(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.press("2 j l l i 9 0 0 enter");
    assert_eq!(ui.budgets().plan_edit, None);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn the_switcher_selection_follows_j_and_k(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.press("shift-b");
    let names = ui.budgets().switcher_names.expect("the switcher is open");
    if names.len() > 1 {
        let start = ui
            .budgets()
            .switcher_selected
            .expect("the switcher is open");
        ui.press("j");
        assert_ne!(ui.budgets().switcher_selected, Some(start));
    }
}

#[gpui::test]
fn tab_steps_the_tabs_and_shift_tab_leaves_the_page(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.press("tab");
    assert_eq!(ui.budgets().tab, "Plan");
    ui.press("tab");
    assert_eq!(ui.budgets().tab, "History");
    ui.press("tab");
    assert_eq!(ui.budgets().tab, "Progress");
    ui.press("shift-tab");
    assert_eq!(ui.budgets().tab, "Progress");
    assert_ne!(
        ui.read(|shell| shell.nav().focus()),
        bin_desktop::navigation::nav::FocusZone::View
    );
}

#[gpui::test]
fn plan_tab_saves_the_cell_and_edits_the_next_month(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.press("2 j l l i 9 0 0 tab");
    let page = ui.budgets();
    assert_eq!(page.plan_cursor, (1, 3));
    assert!(page.plan_edit.is_some());
    assert_eq!(mode(&mut ui), InputMode::Insert);
    ui.press("escape");
    assert_eq!(ui.budgets().plan_edit, None);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn an_insert_the_page_did_not_ask_for_leaves_its_keys_alone(app: &mut TestAppContext) {
    let mut ui = on_budgets(app);

    ui.press("a");
    assert_eq!(mode(&mut ui), InputMode::Insert);
    ui.press("tab 2");
    assert_eq!(ui.budgets().tab, "Progress");
    ui.press("escape");
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

/// The Budget names the Switcher lists, closing it again.
fn budget_names(ui: &mut Harness<'_>) -> Vec<String> {
    ui.press("shift-b");
    let names = ui.budgets().switcher_names.unwrap_or_default();
    ui.press("escape");
    names
}

#[gpui::test]
fn a_failed_budgets_write_puts_the_budgets_back_with_an_error_toast(app: &mut TestAppContext) {
    let mut ui = Harness::with_failing_budgets(app);
    ui.press("g b");
    let stored = budget_names(&mut ui);

    ui.press("n t r i p enter");
    assert_eq!(ui.budgets().dialog, None, "the New budget Dialog confirmed");
    assert!(
        budget_names(&mut ui).contains(&"trip".to_string()),
        "memory runs ahead of the file until the failure is heard"
    );

    ui.settle_writes();

    assert_eq!(budget_names(&mut ui), stored);
    assert!(ui.toasts().visible.into_iter().any(|toast| {
        toast.kind == "Error"
            && toast.text == "Couldn't save the change to budgets, so they are back as last saved"
    }));
}
