//! The Dashboard, the `?` Help overlay, the Toast layer with its history, and the Import step
//! driven by keys. State is read back from `Shell`.

mod common;

use bin_desktop::navigation::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;
use lib_toast::ToastKind;

fn mode(ui: &mut Harness<'_>) -> InputMode {
    ui.read(|shell| shell.nav().mode())
}

fn on_import(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press(": i m p o r t enter");
    assert_eq!(ui.noun(), Noun::Transactions);
    ui
}

#[gpui::test]
fn the_dashboard_is_the_start_page_and_lists_bills_needing_attention(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    assert_eq!(ui.noun(), Noun::Dashboard);

    let bills = ui.dashboard_bills();
    assert!(!bills.is_empty(), "the stub ledger has Bills to attend to");
    assert!(bills.iter().all(|bill| !bill.plan.is_empty()));
}

#[gpui::test]
fn help_opens_with_question_mark_and_escape_closes_it(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    ui.press("?");
    assert_eq!(mode(&mut ui), InputMode::Help);
    ui.press("escape");
    assert_eq!(mode(&mut ui), InputMode::Normal);
    assert_eq!(ui.noun(), Noun::Dashboard);
}

#[gpui::test]
fn a_raised_toast_shows_in_the_stack_and_the_history(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    assert!(ui.toasts().on);
    assert!(ui.toasts().visible.is_empty());

    ui.raise_toast(ToastKind::Success, "Saved");
    let toasts = ui.toasts();
    assert_eq!(toasts.visible.len(), 1);
    assert_eq!(toasts.visible[0].kind, "Success");
    assert_eq!(toasts.visible[0].text, "Saved");
    assert_eq!(toasts.history.len(), 1);
}

#[gpui::test]
fn a_repeated_toast_merges_into_one_with_a_count(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    ui.raise_toast(ToastKind::Info, "Synced");
    ui.raise_toast(ToastKind::Info, "Synced");
    let toasts = ui.toasts();
    assert_eq!(toasts.visible.len(), 1);
    assert_eq!(toasts.visible[0].count, 2);
    assert_eq!(toasts.history.len(), 1);
    assert_eq!(toasts.history[0].count, 2);
}

#[gpui::test]
fn ctrl_l_dismisses_every_toast_but_keeps_the_history(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.raise_toast(ToastKind::Info, "one");
    ui.raise_toast(ToastKind::Warning, "two");
    assert_eq!(ui.toasts().visible.len(), 2);

    ui.press("ctrl-l");
    let toasts = ui.toasts();
    assert!(toasts.visible.is_empty());
    assert_eq!(toasts.history.len(), 2);
}

#[gpui::test]
fn the_dismiss_all_command_clears_the_stack(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.raise_toast(ToastKind::Info, "one");

    ui.press(": d i s m i s s space a l l enter");
    assert!(ui.toasts().visible.is_empty());
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn the_toasts_command_opens_the_history_newest_first_and_escape_closes_it(
    app: &mut TestAppContext,
) {
    let mut ui = Harness::new(app);
    ui.raise_toast(ToastKind::Info, "older");
    ui.raise_toast(ToastKind::Error, "newer");

    ui.press(": m e s s a g e s enter");
    let toasts = ui.toasts();
    assert!(toasts.history_open);
    assert_eq!(mode(&mut ui), InputMode::Dialog);
    assert_eq!(toasts.history[0].text, "newer");
    assert_eq!(toasts.history[1].text, "older");

    ui.press("escape");
    assert!(!ui.toasts().history_open);
    assert_eq!(mode(&mut ui), InputMode::Normal);
    assert_eq!(ui.toasts().visible.len(), 2, "viewing dismissed nothing");
}

#[gpui::test]
fn toasts_off_and_on_set_the_preference(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    ui.press(": t o a s t s space o f f enter");
    assert!(!ui.toasts().on);
    ui.raise_toast(ToastKind::Info, "hidden");
    assert!(ui.toasts().visible.is_empty());

    ui.press(": t o a s t s space o n enter");
    assert!(ui.toasts().on);
}

#[gpui::test]
fn import_opens_on_the_seeded_statement_in_place_of_transactions(app: &mut TestAppContext) {
    let mut ui = on_import(app);

    let import = ui.import().expect("`:import` opens the step");
    assert_eq!(import.rows.len(), 18);
    assert_eq!(import.selected, 0);
    assert!(import.remember);
    assert_eq!(import.open_select, None);
    assert!(
        !import.can_continue,
        "unmatched rows need review before continuing"
    );
}

#[gpui::test]
fn j_and_k_move_the_row_and_r_toggles_remember(app: &mut TestAppContext) {
    let mut ui = on_import(app);

    ui.press("j j");
    assert_eq!(ui.import().expect("import is open").selected, 2);
    ui.press("k");
    assert_eq!(ui.import().expect("import is open").selected, 1);
    ui.press("r");
    assert!(!ui.import().expect("import is open").remember);
    ui.press("r");
    assert!(ui.import().expect("import is open").remember);
}

#[gpui::test]
fn p_opens_the_payee_select_and_escape_closes_only_the_select(app: &mut TestAppContext) {
    let mut ui = on_import(app);

    ui.press("p");
    assert_eq!(
        ui.import().expect("import is open").open_select.as_deref(),
        Some("Payee")
    );
    ui.press("escape");
    let import = ui.import().expect("escape closed the select, not the step");
    assert_eq!(import.open_select, None);
}

#[gpui::test]
fn c_opens_the_category_select(app: &mut TestAppContext) {
    let mut ui = on_import(app);

    ui.press("c");
    assert_eq!(
        ui.import().expect("import is open").open_select.as_deref(),
        Some("Category")
    );
}

#[gpui::test]
fn enter_does_nothing_while_a_row_needs_review(app: &mut TestAppContext) {
    let mut ui = on_import(app);

    ui.press("enter");
    assert!(ui.import().is_some());
    assert!(ui.toasts().history.is_empty());
}

#[gpui::test]
fn escape_leaves_the_import_step(app: &mut TestAppContext) {
    let mut ui = on_import(app);

    ui.press("escape");
    assert_eq!(ui.import(), None);
    assert_eq!(ui.noun(), Noun::Transactions);
}
