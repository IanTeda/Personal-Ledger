//! The Dashboard driven by clicks. Only elements tagged with `debug_selector` are clicked; state
//! is read back from `Root`.

use super::common;

use bin_desktop::navigation::nav::Noun;
use common::Harness;
use gpui::TestAppContext;

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
