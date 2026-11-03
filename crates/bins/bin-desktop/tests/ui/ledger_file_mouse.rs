//! Start-up with no Ledger open, driven by clicks (#977): the muted rail and the This Ledger
//! Settings entries refuse a click on the status line and stay put, while Settings, its This device
//! pages and the Dashboard's `:open` still run. State is read back from `Root`.

use super::common;

use bin_desktop::{
    WindowSize,
    navigation::nav::{InputMode, Noun},
};
use common::Harness;
use gpui::TestAppContext;

#[gpui::test]
fn clicking_a_rail_noun_that_needs_a_ledger_is_refused(app: &mut TestAppContext) {
    let mut ui = Harness::without_ledger(app);

    for noun in ["Transactions", "Bills", "Budgets", "Documents"] {
        ui.click(&format!("primary-rail-{noun}"));
        assert_eq!(ui.noun(), Noun::Dashboard, "{noun} stays muted");
        assert!(ui.refused_without_ledger(), "{noun} says why");
    }
}

#[gpui::test]
fn clicking_settings_on_the_rail_opens_its_this_device_pages(app: &mut TestAppContext) {
    let mut ui = Harness::without_ledger(app);

    ui.click("primary-rail-Settings");
    assert_eq!(ui.noun(), Noun::Settings);
    assert_eq!(ui.settings_frame().page, "General");
    ui.click("settings-index-Tracing");
    assert_eq!(ui.settings_frame().page, "Tracing");
}

#[gpui::test]
fn clicking_a_this_ledger_settings_entry_is_refused(app: &mut TestAppContext) {
    let mut ui = Harness::without_ledger(app);
    ui.click("primary-rail-Settings");

    ui.click("settings-index-Units");
    assert_eq!(
        ui.settings_frame().page,
        "General",
        "the page on show stays"
    );
    assert!(ui.refused_without_ledger());
}

#[gpui::test]
fn picking_a_this_ledger_page_in_the_medium_picker_is_refused(app: &mut TestAppContext) {
    let mut ui = Harness::without_ledger(app);
    ui.resize_to(WindowSize::Medium);
    ui.click("primary-rail-collapsed-Settings");
    ui.click("settings-index-page-Display");
    assert_eq!(ui.settings_frame().page, "Display");

    ui.click("settings-crumb-picker");
    ui.click("settings-picker-Accounts");
    let frame = ui.settings_frame();
    assert_eq!(frame.page, "Display");
    assert!(frame.picker_chosen.is_some(), "the picker stays open");
    assert!(ui.refused_without_ledger());
}

#[gpui::test]
fn clicking_open_on_the_dashboard_opens_the_file_explorer(app: &mut TestAppContext) {
    let mut ui = Harness::without_ledger(app);

    ui.click("no-ledger-open");
    assert_eq!(
        ui.read(|shell| shell.nav().mode()),
        InputMode::Command,
        "the file explorer is open"
    );
    assert!(!ui.refused_without_ledger());
}
