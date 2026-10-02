//! The Settings Data & backup page driven by keys: paging to it and the focus behaviour. State is
//! read back from `Shell`.

mod common;

use common::Harness;
use gpui::TestAppContext;

fn on_page(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    ui.press(&["j"; 9].join(" "));
    assert_eq!(ui.settings().page, "DataBackup");
    ui
}

#[gpui::test]
fn j_pages_to_data_backup_between_sync_server_and_tracing(app: &mut TestAppContext) {
    let mut ui = on_page(app);

    ui.press("k");
    assert_eq!(ui.settings().page, "SyncServer");
    ui.press("j j");
    assert_eq!(ui.settings().page, "Tracing");
}

#[gpui::test]
fn data_backup_takes_focus_with_l_and_escape_returns(app: &mut TestAppContext) {
    let mut ui = on_page(app);

    ui.press("l");
    assert!(ui.settings().page_focused);
    ui.press("escape");
    assert!(!ui.settings().page_focused);
}

#[gpui::test]
fn keys_alone_do_not_trigger_the_buttons(app: &mut TestAppContext) {
    let mut ui = on_page(app);
    ui.press("l");

    ui.press("j k enter");
    assert_eq!(ui.settings().status_message, None);
}
