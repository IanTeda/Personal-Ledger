//! `:data`'s folder picker driven by the mouse (DESKTOP-187, #1148): a folder row walks into it, a
//! path bar segment goes back up, a file row is inert, **Use <folder>** picks the folder browsed
//! and **Cancel** changes nothing. State is read back from `Root` and the conf file.

use super::common;

use bin_desktop::navigation::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;
use lib_shell::ExplorerMode;

#[gpui::test]
fn a_folder_row_walks_in_a_file_row_is_inert_and_use_picks(app: &mut TestAppContext) {
    let dir = common::data_folder();
    let mut ui = Harness::new(app);
    ui.browse_from(dir.path());
    ui.press("g w");
    ui.press(": d a t a enter");
    assert_eq!(
        ui.explorer().map(|picker| picker.mode),
        Some(ExplorerMode::PickFolder)
    );

    // Folders first, then files: archive, finance, expenses.csv, teda.pldb.
    ui.click("explorer-row-3");
    assert_eq!(
        ui.explorer().map(|picker| (picker.folder, picker.selected)),
        Some((dir.path().to_path_buf(), Some(dir.path().join("archive")))),
        "a file is listed for what's there, never picked"
    );
    ui.click("explorer-row-1");
    assert_eq!(
        ui.explorer().map(|picker| picker.folder),
        Some(dir.path().join("finance"))
    );

    ui.click("explorer-open");

    let finance = dir.path().join("finance");
    assert_eq!(ui.explorer(), None);
    assert_eq!(ui.noun(), Noun::Bills);
    assert_eq!(
        ui.conf(),
        format!("[General]\ndata = {}\n", finance.display())
    );
    assert_eq!(
        ui.toasts().visible.pop().map(|toast| toast.text),
        Some(format!("Data folder is now {}", finance.display()))
    );
}

#[gpui::test]
fn cancel_closes_the_picker_with_nothing_changed(app: &mut TestAppContext) {
    let dir = common::data_folder();
    let mut ui = Harness::without_ledger(app);
    ui.browse_from(dir.path());
    ui.press(": d a t a enter");
    ui.click("explorer-row-0");

    ui.click("explorer-cancel");

    assert_eq!(ui.explorer(), None);
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Normal);
    assert_eq!(ui.conf(), "");
}
