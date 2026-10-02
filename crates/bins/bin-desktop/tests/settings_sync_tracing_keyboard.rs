//! The Settings General, Sync server and Tracing pages driven by keys: paging to each and the
//! focus behaviour. State is read back from `Shell`.

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
    let mut ui = on_page(app, 8, "SyncServer");

    ui.press("j j");
    assert_eq!(ui.settings().page, "Tracing");
    ui.press("k");
    assert_eq!(ui.settings().page, "DataBackup");
}

#[gpui::test]
fn general_sync_server_and_tracing_take_focus_with_l(app: &mut TestAppContext) {
    for (downs, page) in [(0, "General"), (8, "SyncServer"), (10, "Tracing")] {
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

#[gpui::test]
fn h_steps_back_out_of_the_tracing_page(app: &mut TestAppContext) {
    let mut ui = on_page(app, 10, "Tracing");

    ui.press("enter");
    assert!(ui.settings().page_focused);
    ui.press("h");
    assert!(!ui.settings().page_focused);
}

#[gpui::test]
fn keys_alone_leave_the_tracing_state_untouched(app: &mut TestAppContext) {
    let mut ui = on_page(app, 10, "Tracing");
    ui.press("l");

    ui.press("j k enter");
    let page = ui.settings();
    assert_eq!(page.tracing_level, "Error");
    assert_eq!(page.log_line_count, 4);
}
