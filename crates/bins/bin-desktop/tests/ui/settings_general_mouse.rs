//! Settings › General driven by the mouse: the On start and Toasts options, and the read-only Locale, Ledger folder and THIS DEVICE box drawn beside them. State is read back from `Root`.

use super::common;
use bin_desktop::shell::router::SettingsGeneralSnapshot;

use common::Harness;
use gpui::TestAppContext;
use lib_core::OnStart;

fn on_general(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    assert_eq!(ui.settings_frame().page, "General");
    ui
}

#[gpui::test]
fn clicking_the_on_start_options_sets_on_start(app: &mut TestAppContext) {
    let mut ui = on_general(app);

    ui.click("general-on-start-1");
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().on_start,
        OnStart::NoLedger
    );
    ui.click("general-on-start-0");
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().on_start,
        OnStart::ReopenLastLedger
    );
}

#[gpui::test]
fn clicking_the_toasts_options_turns_them_off_and_on(app: &mut TestAppContext) {
    let mut ui = on_general(app);

    ui.click("general-toasts-1");
    assert!(!ui.settings::<SettingsGeneralSnapshot>().toasts_on);
    assert!(!ui.toasts().on);
    ui.click("general-toasts-0");
    assert!(ui.settings::<SettingsGeneralSnapshot>().toasts_on);
    assert!(ui.toasts().on);
}

#[gpui::test]
fn the_locale_ledger_folder_and_this_device_box_are_drawn(app: &mut TestAppContext) {
    let mut ui = on_general(app);

    assert!(ui.is_drawn("general-locale"));
    assert!(ui.is_drawn("general-ledger-folder"));
    assert!(ui.is_drawn("general-this-device"));
    // A test Shell is told nothing of this device, so THIS DEVICE reads none.
    let general = ui.settings::<SettingsGeneralSnapshot>();
    assert_eq!(general.last_ledger, None);
    assert_eq!(general.configuration, None);
}

#[gpui::test]
fn toasts_are_no_longer_on_display(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("g s j");
    assert_eq!(ui.settings_frame().page, "Display");

    assert!(ui.is_drawn("display-fields"));
    assert!(!ui.is_drawn("display-toasts-0"));
}
