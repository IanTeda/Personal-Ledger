//! `:data` driven by keys (DESKTOP-189, #1148): the palette's **Change data folder** opens the file
//! explorer in its folder-picking mode over whatever View is showing, with or without a Ledger
//! open. `j`/`k` move over the folders, `l` or `enter` walks into one, `h` or `backspace` goes up,
//! and `ctrl-enter` picks the folder browsed: it is written to `[General] data`, raises a Success
//! Toast and applies at once, so the next `:open` starts there. `esc` changes nothing. Either way
//! the picker returns to the View as it was, the Open Ledger still open. State is read back from
//! `Root` and the conf file.

#![expect(
    clippy::expect_used,
    reason = "test helpers: a folder that can't be made or a picker that isn't open should fail the test loudly"
)]

use super::common;
use lib_config::KeyBindingConfig;

use bin_desktop::navigation::nav::{InputMode, Noun};
use bin_desktop::shell::router::SettingsGeneralSnapshot;
use common::Harness;
use gpui::TestAppContext;
use lib_shell::ExplorerMode;

/// `:data` run from the Dashboard, the picker open in `dir`.
fn picking_from_the_dashboard(ui: &mut Harness<'_>, dir: &std::path::Path) {
    ui.browse_from(dir);
    ui.press("g d");
    ui.press(": d a t a enter");
    let picker = ui.explorer().expect("the picker is open");
    assert_eq!(picker.mode, ExplorerMode::PickFolder);
    assert_eq!(picker.folder, dir);
    assert_eq!(picker.current.as_deref(), Some(dir), "the header names it");
    assert_eq!(
        picker.selected,
        Some(dir.join("archive")),
        "the cursor starts on the first folder"
    );
    assert_eq!(ui.noun(), Noun::Dashboard);
    assert!(ui.is_drawn("explorer-pick-note"));
    assert!(!ui.is_drawn("explorer-filter-pldb-only"));
}

/// Walks into `finance` and picks it, then checks it is written, in effect and toasted.
fn use_finance(ui: &mut Harness<'_>, dir: &std::path::Path) {
    ui.press("j l");
    assert_eq!(
        ui.explorer().map(|picker| picker.folder),
        Some(dir.join("finance"))
    );
    ui.press("ctrl-enter");

    let finance = dir.join("finance");
    assert_eq!(ui.explorer(), None, "Use closes the picker");
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Normal);
    assert_eq!(ui.noun(), Noun::Dashboard, "back on the View it came from");
    assert_eq!(
        ui.conf(),
        format!("[General]\ndata = {}\n", finance.display())
    );
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().data_folder,
        finance.display().to_string()
    );
    let toast = ui.toasts().visible.pop().expect("a Success Toast");
    assert_eq!(toast.kind, "Success");
    assert_eq!(
        toast.text,
        format!("Data folder is now {}", finance.display())
    );

    ui.press(": o p e n enter");
    assert_eq!(
        ui.explorer()
            .map(|explorer| (explorer.mode, explorer.folder)),
        Some((ExplorerMode::Open, finance)),
        ":open starts in the new Data folder"
    );
}

#[gpui::test]
fn data_with_a_ledger_open_picks_the_folder_and_keeps_the_ledger(app: &mut TestAppContext) {
    let dir = common::data_folder();
    let mut ui = Harness::new(app);
    let ledger = ui.read(|shell| shell.ledger_open());
    assert!(ledger);

    picking_from_the_dashboard(&mut ui, dir.path());
    use_finance(&mut ui, dir.path());

    assert!(ui.read(|shell| shell.ledger_open()), "nothing moves");
}

#[gpui::test]
fn data_with_no_ledger_open_picks_the_folder(app: &mut TestAppContext) {
    let dir = common::data_folder();
    let mut ui = Harness::without_ledger(app);

    picking_from_the_dashboard(&mut ui, dir.path());
    assert!(!ui.refused_without_ledger());
    use_finance(&mut ui, dir.path());
}

#[gpui::test]
fn h_and_backspace_go_up_with_the_cursor_on_the_folder_left(app: &mut TestAppContext) {
    let dir = common::data_folder();
    let mut ui = Harness::new(app);
    picking_from_the_dashboard(&mut ui, dir.path());

    ui.press("j enter");
    assert_eq!(
        ui.explorer().map(|picker| picker.folder),
        Some(dir.path().join("finance"))
    );
    ui.press("h");
    let picker = ui.explorer().expect("still open");
    assert_eq!(picker.folder, dir.path());
    assert_eq!(picker.selected, Some(dir.path().join("finance")));

    ui.press("l backspace");
    assert_eq!(
        ui.explorer().map(|picker| picker.folder),
        Some(dir.path().to_path_buf())
    );
}

#[gpui::test]
fn rebound_go_into_and_go_up_keys_walk_the_folders(app: &mut TestAppContext) {
    let dir = common::data_folder();
    let mut config = KeyBindingConfig::overriding("go_into", "right");
    config
        .bindings
        .insert("go_up".to_string(), "left".to_string());
    let mut ui = Harness::with_keybindings(app, config);
    picking_from_the_dashboard(&mut ui, dir.path());

    ui.press("j right");
    assert_eq!(
        ui.explorer().map(|picker| picker.folder),
        Some(dir.path().join("finance"))
    );
    ui.press("left");
    assert_eq!(
        ui.explorer().map(|picker| picker.folder),
        Some(dir.path().to_path_buf())
    );

    ui.press("j l");
    ui.press("backspace");
    assert_eq!(
        ui.explorer().map(|picker| picker.folder),
        Some(dir.path().to_path_buf()),
        "the replaced defaults no longer walk"
    );
}

#[gpui::test]
fn escape_closes_the_picker_with_nothing_changed(app: &mut TestAppContext) {
    let dir = common::data_folder();
    let mut ui = Harness::with_configuration(app, &common::browsing(dir.path()), &[]);
    ui.press("g w");
    ui.press(": d a t a enter");
    ui.press("l");

    ui.press("escape");

    assert_eq!(ui.explorer(), None);
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Normal);
    assert_eq!(ui.noun(), Noun::Bills, "back on the View it came from");
    assert_eq!(ui.conf(), "", "nothing written");
    assert!(ui.toasts().visible.is_empty());
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().data_folder,
        dir.path().display().to_string()
    );
}

#[gpui::test]
fn using_the_folder_already_set_writes_nothing_and_raises_nothing(app: &mut TestAppContext) {
    let dir = common::data_folder();
    let mut ui = Harness::with_configuration(app, &common::browsing(dir.path()), &[]);
    ui.press(": d a t a enter");

    ui.press("ctrl-enter");

    assert_eq!(ui.explorer(), None, "Use still closes the picker");
    assert_eq!(ui.conf(), "");
    assert!(ui.toasts().visible.is_empty());
}
