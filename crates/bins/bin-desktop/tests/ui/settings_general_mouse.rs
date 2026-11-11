//! Settings › General driven by the mouse: the On start, Start view and Toasts options, each saved to the conf file (ADR-0053), and the read-only Locale, Ledger folder and THIS DEVICE box drawn beside them. State is read back from `Root` and the conf file.

use super::common;
use bin_desktop::shell::router::SettingsGeneralSnapshot;

use bin_desktop::WindowSize;
use common::Harness;
use gpui::TestAppContext;
use lib_core::OnStart;

fn on_general(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s enter");
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
    assert_eq!(ui.conf(), "[General]\non_start = none\n");
    ui.click("general-on-start-0");
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().on_start,
        OnStart::ReopenLastLedger
    );
}

#[gpui::test]
fn clicking_a_start_view_option_sets_start_view(app: &mut TestAppContext) {
    let mut ui = on_general(app);
    assert!(!ui.is_drawn("general-start-view-unlisted"));

    ui.click("general-start-view-4");
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().start_view,
        "documents"
    );
    assert_eq!(ui.conf(), "[General]\nstart_view = documents\n");
    ui.click("general-start-view-2");
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().start_view,
        "settings"
    );
    assert_eq!(ui.conf(), "[General]\nstart_view = settings\n");
}

#[gpui::test]
fn clicking_the_toasts_options_turns_them_off_and_on(app: &mut TestAppContext) {
    let mut ui = on_general(app);

    ui.click("general-toasts-1");
    assert!(!ui.settings::<SettingsGeneralSnapshot>().toasts_on);
    assert!(!ui.toasts().on);
    assert_eq!(ui.conf(), "[General]\ntoasts = false\n");
    ui.click("general-toasts-0");
    assert!(ui.settings::<SettingsGeneralSnapshot>().toasts_on);
    assert!(ui.toasts().on);
}

#[gpui::test]
fn clicking_a_shadowed_toasts_option_changes_nothing(app: &mut TestAppContext) {
    let mut ui = Harness::with_configuration(
        app,
        &lib_config::Config::default(),
        &[("PERSONAL_LEDGER_GENERAL__TOASTS", "true")],
    );
    ui.press("g s enter");
    assert!(ui.is_drawn("shadowed-toasts"));

    ui.click("general-toasts-1");

    assert!(ui.settings::<SettingsGeneralSnapshot>().toasts_on);
    assert!(ui.toasts().on);
    assert_eq!(ui.conf(), "");
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
    ui.press("g s j enter");
    assert_eq!(ui.settings_frame().page, "Display");

    assert!(ui.is_drawn("display-fields"));
    assert!(!ui.is_drawn("display-toasts-0"));
}

#[gpui::test]
fn the_fields_and_this_device_box_each_take_half_the_view(app: &mut TestAppContext) {
    for size in [WindowSize::Medium, WindowSize::Large, WindowSize::XLarge] {
        let mut ui = on_general(app);
        ui.resize_to(size);
        let page = ui.bounds("settings-page");
        let fields = ui.bounds("general-fields");
        let device = ui.bounds("general-this-device");
        let half = (page.size.width - gpui::px(40.0)) / 2.0;
        let close = |a: gpui::Pixels, b: gpui::Pixels| (a - b).abs() <= gpui::px(1.0);
        assert!(
            close(fields.size.width, half),
            "{size:?}: fields {fields:?} of {page:?}"
        );
        assert!(
            close(device.size.width, half),
            "{size:?}: box {device:?} of {page:?}"
        );
        assert!(
            device.origin.x >= fields.origin.x + fields.size.width,
            "{size:?}: side by side"
        );
        assert!(
            close(device.origin.y, fields.origin.y)
                || device.origin.y < fields.origin.y + gpui::px(40.0),
            "{size:?}: same row"
        );
    }
}
