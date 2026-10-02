//! The Settings Data & backup page driven by clicks: Backup now and Export ledger. Both are
//! stand-ins with no OS call behind them, so the test asserts only that each reports in the status
//! line. Only elements tagged with `debug_selector` are clicked.

mod common;

use common::Harness;
use gpui::TestAppContext;

fn on_page(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    ui.click("settings-index-DataBackup");
    assert_eq!(ui.settings().page, "DataBackup");
    assert_eq!(ui.settings().status_message, None);
    ui
}

#[gpui::test]
fn backup_now_reports_in_the_status_line(app: &mut TestAppContext) {
    let mut ui = on_page(app);

    ui.click("settings-backup-now");
    let message = ui.settings().status_message.expect("a status message");
    assert!(message.to_lowercase().contains("backup"), "{message}");
}

#[gpui::test]
fn export_ledger_reports_in_the_status_line(app: &mut TestAppContext) {
    let mut ui = on_page(app);

    ui.click("settings-export-ledger");
    let message = ui.settings().status_message.expect("a status message");
    assert!(message.to_lowercase().contains("export"), "{message}");
}
