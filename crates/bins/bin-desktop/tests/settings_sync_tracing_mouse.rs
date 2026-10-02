//! The Settings Sync server and Tracing pages driven by clicks: Sync now, the log level radios
//! and Clear logs. Only elements tagged with `debug_selector` are clicked.

mod common;

use common::Harness;
use gpui::TestAppContext;

fn on_page<'a>(app: &'a mut TestAppContext, section: &str) -> Harness<'a> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    ui.click(&format!("settings-index-{section}"));
    assert_eq!(ui.settings().page, section);
    ui
}

#[gpui::test]
fn sync_now_reports_in_the_status_line_without_syncing(app: &mut TestAppContext) {
    let mut ui = on_page(app, "SyncServer");
    assert_eq!(ui.settings().status_message, None);

    ui.click("settings-sync-now");
    let message = ui.settings().status_message.expect("a status message");
    assert!(message.to_lowercase().contains("sync"), "{message}");
}

#[gpui::test]
fn clicking_a_level_radio_selects_it(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Tracing");
    assert_eq!(ui.settings().tracing_level, "Error");

    ui.click("tracing-level-debug");
    assert_eq!(ui.settings().tracing_level, "Debug");
    ui.click("tracing-level-warn");
    assert_eq!(ui.settings().tracing_level, "Warn");
    ui.click("tracing-level-info");
    assert_eq!(ui.settings().tracing_level, "Info");
    ui.click("tracing-level-error");
    assert_eq!(ui.settings().tracing_level, "Error");
}

#[gpui::test]
fn clear_logs_empties_the_viewport_and_keeps_the_level(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Tracing");
    ui.click("tracing-level-info");
    assert_eq!(ui.settings().log_line_count, 4);

    ui.click("settings-clear-logs");
    let page = ui.settings();
    assert_eq!(page.log_line_count, 0);
    assert_eq!(page.tracing_level, "Info");
    ui.click("settings-clear-logs");
    assert_eq!(ui.settings().log_line_count, 0);
}
