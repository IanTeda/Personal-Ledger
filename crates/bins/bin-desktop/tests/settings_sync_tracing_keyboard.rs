//! The Settings General, Sync server and Tracing pages driven by keys: paging to each and the
//! focus behaviour. State is read back from `Shell`. The Tracing page's own keys are in
//! `settings_tracing_keyboard.rs`.

mod common;

use common::Harness;
use gpui::TestAppContext;

fn on_page<'a>(app: &'a mut TestAppContext, downs: usize, page: &str) -> Harness<'a> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    if downs > 0 {
        ui.press(&vec!["j"; downs].join(" "));
    }
    assert_eq!(ui.settings().page, page);
    ui
}

#[gpui::test]
fn j_pages_to_sync_server_and_tracing(app: &mut TestAppContext) {
    let mut ui = on_page(app, 10, "SyncServer");

    ui.press("j j");
    assert_eq!(ui.settings().page, "Tracing");
    ui.press("k");
    assert_eq!(ui.settings().page, "DataBackup");
}

#[gpui::test]
fn general_sync_server_and_tracing_take_focus_with_l(app: &mut TestAppContext) {
    for (downs, page) in [(0, "General"), (10, "SyncServer"), (12, "Tracing")] {
        let mut ui = on_page(app, downs, page);
        ui.press("l");
        assert!(ui.settings().page_focused, "{page} should take focus");
        ui.press("escape");
        assert!(
            !ui.settings().page_focused,
            "{page} escape returns to the index"
        );
    }
}
