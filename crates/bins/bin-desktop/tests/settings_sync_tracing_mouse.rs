//! The Settings Sync server page driven by clicks: Sync now. Only elements tagged with
//! `debug_selector` are clicked. The Tracing page's clicks are in `settings_tracing_mouse.rs`.

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
