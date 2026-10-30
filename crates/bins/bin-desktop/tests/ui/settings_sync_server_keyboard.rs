//! The Settings Sync server page driven by keys: paging to it, and stepping into and out of it.
//! The page is a read-only column with one stand-in button, so it has no keys of its own. State is
//! read back from `Root`.

use super::common;

use common::Harness;
use gpui::TestAppContext;

fn on_sync_server(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    ui.press(&["j"; 10].join(" "));
    assert_eq!(ui.settings_frame().page, "SyncServer");
    ui
}

#[gpui::test]
fn j_and_k_page_past_sync_server_to_its_neighbours(app: &mut TestAppContext) {
    let mut ui = on_sync_server(app);

    ui.press("j");
    assert_eq!(ui.settings_frame().page, "DataBackup");
    ui.press("k k");
    assert_eq!(ui.settings_frame().page, "Inventory");
}

#[gpui::test]
fn l_steps_into_sync_server_and_escape_steps_back_out(app: &mut TestAppContext) {
    let mut ui = on_sync_server(app);

    ui.press("l");
    assert!(ui.settings_frame().page_focused);
    ui.press("escape");
    let page = ui.settings_frame();
    assert!(!page.page_focused);
    assert_eq!(page.page, "SyncServer");
}
