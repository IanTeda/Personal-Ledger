//! The Dashboard, the `?` Help overlay, the Toast layer with its history, and the Import step
//! driven by clicks. Only elements tagged with `debug_selector` are clicked; state is read back
//! from `Shell`.

mod common;

use bin_desktop::navigation::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;
use lib_toast::ToastKind;

fn on_import(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press(": i m p o r t enter");
    assert!(ui.import().is_some());
    ui
}

#[gpui::test]
fn clicking_a_dashboard_bill_opens_it_on_the_bills_schedule(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.shell.update(ui.cx, |shell, cx| {
        shell.open_ledger_for_test();
        cx.notify();
    });
    ui.cx.run_until_parked();
    let bills = ui.dashboard_bills();
    let first = bills
        .first()
        .expect("the stub ledger has a Bill to attend to");

    ui.click(&first.selector);
    assert_eq!(ui.noun(), Noun::Bills);
    let page = ui.bills();
    assert_eq!(page.tab, "Schedule");
    assert_eq!(page.rows[page.selected].plan, first.plan);
}

#[gpui::test]
fn the_help_close_button_leaves_help_mode(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("?");
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Help);

    ui.click("help-close");
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Normal);
}

#[gpui::test]
fn a_toasts_cross_dismisses_just_that_toast(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.raise_toast(ToastKind::Info, "first");
    ui.raise_toast(ToastKind::Success, "second");

    ui.click("toast-dismiss-0");
    let toasts = ui.toasts();
    assert_eq!(toasts.visible.len(), 1);
    assert_eq!(toasts.visible[0].text, "second");
    assert_eq!(
        toasts.history.len(),
        2,
        "the history keeps what was dismissed"
    );
}

#[gpui::test]
fn the_history_close_button_shuts_it(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.raise_toast(ToastKind::Info, "one");
    ui.press(": m e s s a g e s enter");
    assert!(ui.toasts().history_open);

    ui.click("toast-history-close");
    assert!(!ui.toasts().history_open);
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Normal);
}

#[gpui::test]
fn clicking_an_import_row_selects_it(app: &mut TestAppContext) {
    let mut ui = on_import(app);

    ui.click("import-row-3");
    assert_eq!(ui.import().expect("import is open").selected, 3);
}

#[gpui::test]
fn clicking_remember_toggles_it(app: &mut TestAppContext) {
    let mut ui = on_import(app);

    ui.click("import-remember");
    assert!(!ui.import().expect("import is open").remember);
    ui.click("import-remember");
    assert!(ui.import().expect("import is open").remember);
}

#[gpui::test]
fn clicking_back_leaves_the_import_step(app: &mut TestAppContext) {
    let mut ui = on_import(app);

    ui.click("import-back");
    assert_eq!(ui.import(), None);
    assert_eq!(ui.noun(), Noun::Transactions);
}
