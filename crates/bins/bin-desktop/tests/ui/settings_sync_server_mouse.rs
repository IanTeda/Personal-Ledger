//! The Settings Sync server page driven by clicks: its index entry and Sync now. Only elements
//! tagged with `debug_selector` are clicked; state is read back from `Root`.

use super::common;

use common::Harness;
use gpui::TestAppContext;

fn on_sync_server(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    ui.click("settings-index-SyncServer");
    assert_eq!(ui.settings().page, "SyncServer");
    ui
}

#[gpui::test]
fn clicking_the_sync_server_index_entry_swaps_to_the_page(app: &mut TestAppContext) {
    let mut ui = on_sync_server(app);

    assert!(!ui.settings().page_focused);
    assert!(ui.is_drawn("settings-sync-now"));
}

#[gpui::test]
fn sync_now_reports_in_the_status_line_without_syncing(app: &mut TestAppContext) {
    let mut ui = on_sync_server(app);
    assert_eq!(ui.settings().status_message, None);

    ui.click("settings-sync-now");
    let message = ui.settings().status_message.expect("a status message");
    assert!(message.to_lowercase().contains("sync"), "{message}");
}
