//! The Settings Data & backup page driven by clicks: Backup now and Export ledger. Both are
//! stand-ins with no OS call behind them, so the test asserts only that each reports in the status
//! line. Only elements tagged with `debug_selector` are clicked.

use super::common;

use common::Harness;
use gpui::TestAppContext;

fn on_page(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    ui.click("settings-index-DataBackup");
    assert_eq!(ui.settings_frame().page, "DataBackup");
    assert_eq!(ui.settings_frame().status_message, None);
    ui
}

#[gpui::test]
fn backup_now_reports_in_the_status_line(app: &mut TestAppContext) {
    let mut ui = on_page(app);

    ui.click("settings-backup-now");
    let message = ui
        .settings_frame()
        .status_message
        .expect("a status message");
    assert!(message.to_lowercase().contains("backup"), "{message}");
}

#[gpui::test]
fn export_ledger_reports_in_the_status_line(app: &mut TestAppContext) {
    let mut ui = on_page(app);

    ui.click("settings-export-ledger");
    let message = ui
        .settings_frame()
        .status_message
        .expect("a status message");
    assert!(message.to_lowercase().contains("export"), "{message}");
}

#[gpui::test]
fn remove_all_api_keys_asks_first_and_cancel_keeps_the_keys(app: &mut TestAppContext) {
    let mut ui = on_page(app);

    ui.click("settings-remove-api-keys");
    assert_eq!(ui.dialog(), Some("RemoveApiKeys"));
    ui.click("remove-api-keys-cancel");

    assert_eq!(ui.dialog(), None);
    assert!(ui.toast_texts().is_empty());
}

#[gpui::test]
fn the_confirm_button_removes_every_key_and_says_so(app: &mut TestAppContext) {
    let mut ui = on_page(app);

    ui.click("settings-remove-api-keys");
    ui.click("remove-api-keys-confirm");

    assert_eq!(ui.dialog(), None);
    assert_eq!(
        ui.toast_texts().first().map(String::as_str),
        Some("API keys removed")
    );
}
