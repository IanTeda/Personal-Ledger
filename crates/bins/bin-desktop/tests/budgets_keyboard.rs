//! The Budgets page driven by keys: tabs, the period and range navs, row selection, the Plan grid
//! and its cell editor, and the detail, limit, stop, fill, switcher and new-budget dialogs. State
//! is read back from `Shell`.

mod common;

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
