//! The Settings Sync server page driven by keys: paging to it, and stepping into and out of it.
//! The page is a read-only column with one stand-in button, so it has no keys of its own. State is
//! read back from `Root`.

use super::common;
use lib_config::KeyBindingConfig;

use common::Harness;
use gpui::TestAppContext;

fn on_sync_server(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    ui.press(&["j"; 13].join(" "));
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

/// The plain pages' legend reads the frame's keys under `[keybindings]`: a rebound move and
/// focus-index key show their new chords where the defaults `j/k` and `h` stood.
#[gpui::test]
fn a_rebound_frame_key_shows_in_the_plain_page_legend(app: &mut TestAppContext) {
    let mut config = KeyBindingConfig::overriding("move_down", "ctrl+j");
    config
        .bindings
        .insert("settings.focus_index".to_string(), "H".to_string());
    let mut ui = Harness::with_keybindings(app, config);
    ui.press("g s");
    ui.press(&["ctrl-j"; 13].join(" "));
    assert_eq!(ui.settings_frame().page, "SyncServer");

    ui.press("l");
    assert!(ui.settings_frame().page_focused);
    let keys: Vec<String> = ui
        .status()
        .map(|status| status.hints.into_iter().map(|(key, _)| key).collect())
        .unwrap_or_default();
    assert_eq!(keys, ["ctrl+j/k", "H"]);
}
