//! The Bills page driven by keys, on the in-memory ports: each key reaches the `lib-bills`
//! Presenter through the Bills keymap or the router, and what it changes is drawn. The rules behind
//! each (which rows show, what Pay and Skip refuse, generation) are the Presenters' tests in
//! `lib-bills`; these check the wiring, the Dialogs the Router opens and the status line.

use super::common;

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
fn the_arrows_home_end_and_the_page_keys_move_through_the_bills_keymap(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    ui.press("down down");
    assert_eq!(ui.bills().selected, 2);
    ui.press("up");
    assert_eq!(ui.bills().selected, 1);
    ui.press("end");
    assert_eq!(ui.bills().selected, 6);
    ui.press("home");
    assert_eq!(ui.bills().selected, 0);
    ui.press("pagedown");
    assert!(ui.bills().selected > 0);
    ui.press("pageup");
    assert_eq!(ui.bills().selected, 0);
}

#[gpui::test]
fn an_override_of_home_moves_it_and_frees_home(app: &mut TestAppContext) {
    let mut ui = Harness::with_keybindings(app, common::overriding("home", "H"));
    ui.press("g w j j");

    ui.press("home");
    assert_eq!(ui.bills().selected, 2, "home is no longer bound");
    ui.press("H");
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
    ui.press("0");
    assert!(!ui.bills().all);
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

/// #460: `Esc` closes an open filter list first, then leaves the filter row.
#[gpui::test]
fn escape_closes_an_open_filter_list_then_leaves_the_filter_row(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    ui.press("f enter");
    ui.press("escape");
    assert_eq!(ui.bills().filter_focus.as_deref(), Some("Plan"));
    ui.press("escape");
    assert_eq!(ui.bills().filter_focus, None);

    ui.press("f escape");
    assert_eq!(ui.bills().filter_focus, None);
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Normal);
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
    assert_eq!(
        ui.read(|s| s.status_message()).as_deref(),
        Some("this bill isn't actionable")
    );

    ui.press("s");
    assert_eq!(ui.bills().dialog, None);
}

#[gpui::test]
fn the_status_line_draws_each_tabs_legend_and_count(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    let status = ui.status().expect("Bills has a page status");
    let keys: Vec<&str> = status.hints.iter().map(|(key, _)| key.as_str()).collect();
    assert_eq!(
        keys,
        ["j/k", "p", "s", "[/]", "0", "1\u{2013}5", "f", "tab"]
    );
    assert!(status.right.ends_with("7 of 7 entries"), "{}", status.right);

    ui.press("tab");
    let status = ui.status().expect("Bills has a page status");
    let keys: Vec<&str> = status.hints.iter().map(|(key, _)| key.as_str()).collect();
    assert_eq!(keys, ["j/k", "e", "n", "tab"]);
    assert_eq!(status.right, "10 bill plans");
}

#[gpui::test]
fn enter_on_a_paid_row_opens_its_transaction(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    // The last row is the Paid one.
    ui.press("shift-g enter");
    assert_eq!(ui.noun(), Noun::Transactions);
}

#[gpui::test]
fn slash_is_inert_since_bills_has_no_search(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    ui.press("/");
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Normal);
    assert_eq!(ui.noun(), Noun::Bills);
}

#[gpui::test]
fn n_opens_the_add_dialog_and_escape_closes_it(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    ui.press("n");
    assert_eq!(ui.bills().dialog.as_deref(), Some("Add"));
    assert_eq!(ui.dialog_value("title").as_deref(), Some("Add bill plan"));
    assert!(ui.is_drawn("bill-plan-name"));
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

#[gpui::test]
fn a_failed_bills_write_puts_the_schedule_back_with_an_error_toast(app: &mut TestAppContext) {
    let mut ui = Harness::with_failing_bills(app);
    ui.press("g w");
    let stored = ui.bills().rows;

    ui.press("s enter");
    assert_ne!(
        ui.bills().rows,
        stored,
        "memory runs ahead of the file until the failure is heard"
    );

    ui.settle_writes();

    assert_eq!(ui.bills().rows, stored);
    assert!(ui.toasts().visible.into_iter().any(|toast| {
        toast.kind == "Error"
            && toast.text == "Couldn't save the change to bills, so they are back as last saved"
    }));
}

#[gpui::test]
fn escape_closes_an_open_select_in_the_plan_form_then_cancels_it(app: &mut TestAppContext) {
    let mut ui = on_bills(app);
    ui.press("n tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Category"));
    ui.press("enter");
    assert_eq!(ui.dialog_value("select_open").as_deref(), Some("true"));

    ui.press("escape");
    assert_eq!(ui.bills().dialog.as_deref(), Some("Add"));
    assert_eq!(ui.dialog_value("select_open").as_deref(), Some("false"));

    ui.press("escape");
    assert_eq!(ui.bills().dialog, None);
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Normal);
}

#[gpui::test]
fn enter_confirms_only_a_valid_plan_form(app: &mut TestAppContext) {
    let mut ui = on_bills(app);
    ui.press("n enter");
    assert_eq!(
        ui.bills().dialog.as_deref(),
        Some("Add"),
        "an unnamed plan can't confirm"
    );
}

#[gpui::test]
fn ctrl_enter_saves_the_plan_form_from_a_select(app: &mut TestAppContext) {
    let mut ui = on_bills(app);
    ui.press("tab e");
    ui.press("space x tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Category"));
    ui.press("enter");
    assert_eq!(
        ui.bills().dialog.as_deref(),
        Some("Edit"),
        "Enter on a select opens it"
    );

    ui.press("ctrl-enter");

    let page = ui.bills();
    assert_eq!(page.dialog, None);
    assert!(
        page.plans
            .contains(&"Car Insurance \u{2014} AAMI x".to_string())
    );
}

#[gpui::test]
fn an_override_of_next_field_moves_the_plan_forms_field(app: &mut TestAppContext) {
    let mut ui = Harness::with_keybindings(app, common::overriding("next_field", "ctrl+n"));
    ui.press("g w n");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Name"));

    ui.press("ctrl-n");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Category"));
    ui.press("tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Category"));
}

#[gpui::test]
fn question_mark_lists_the_schedules_keys_the_status_line_shows(app: &mut TestAppContext) {
    let mut ui = on_bills(app);

    ui.assert_cheat_sheet_matches_status("Bills keys");
}

#[gpui::test]
fn question_mark_lists_the_planners_keys_the_status_line_shows(app: &mut TestAppContext) {
    let mut ui = on_bills(app);
    ui.press("tab");
    assert_eq!(ui.bills().tab, "Planner");

    ui.assert_cheat_sheet_matches_status("Bills keys");
}

#[gpui::test]
fn the_cheat_sheet_shows_a_rebound_bills_key(app: &mut TestAppContext) {
    let mut ui = Harness::with_keybindings(app, common::overriding("bills.pay", "ctrl+p"));
    ui.press("g w");
    assert!(
        ui.status_as_shortcuts()
            .iter()
            .any(|(_, keys)| keys == "ctrl+p"),
        "the status line shows the rebound key"
    );

    ui.assert_cheat_sheet_matches_status("Bills keys");
}
