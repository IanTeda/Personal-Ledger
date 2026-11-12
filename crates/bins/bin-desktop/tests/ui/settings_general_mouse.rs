//! Settings › General driven by the mouse: the On start and Toasts options and the Start view and Locale dropdowns (#1156), each saved to the conf file (ADR-0053), the Locale even under an override (DESKTOP-186), Toasts' Test raising the four test Toasts, the Data folder's **Change…** opening the folder picker, whose **Use <folder>** writes it (DESKTOP-187, DESKTOP-188), and the THIS DEVICE box drawn beside them. State is read back from `Root` and the conf file.

use super::common;
use bin_desktop::shell::router::SettingsGeneralSnapshot;

use bin_desktop::WindowSize;
use common::Harness;
use gpui::TestAppContext;
use lib_core::OnStart;
use std::time::Duration;

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
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().start_view_list,
        None,
        "closed"
    );

    ui.click("general-start-view");
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().start_view_list,
        Some(0),
        "open"
    );
    ui.click("general-start-view-option-4");
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().start_view_list,
        None,
        "a pick closes it"
    );
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().start_view,
        "documents"
    );
    assert_eq!(ui.conf(), "[General]\nstart_view = documents\n");
    ui.click("general-start-view");
    ui.click("general-start-view-option-2");
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().start_view,
        "settings"
    );
    assert_eq!(ui.conf(), "[General]\nstart_view = settings\n");
}

#[gpui::test]
fn a_second_click_on_a_dropdown_closes_it_and_opening_one_closes_the_other(
    app: &mut TestAppContext,
) {
    let mut ui = on_general(app);

    ui.click("general-start-view");
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().start_view_list,
        Some(0)
    );
    ui.click("general-locale-select");
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().start_view_list,
        None,
        "one list at a time"
    );
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().locale_list,
        Some(0)
    );
    ui.click("general-locale-select");
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().locale_list,
        None,
        "cancelled"
    );
    assert_eq!(ui.conf(), "", "nothing written");
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
fn clicking_a_locale_option_writes_it_and_system_takes_it_out(app: &mut TestAppContext) {
    let mut ui = on_general(app);
    assert!(!ui.is_drawn("general-locale-note"));

    ui.click("general-locale-select");
    ui.click("general-locale-select-option-2");
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().locale.as_deref(),
        Some("en-GB")
    );
    assert_eq!(ui.conf(), "[General]\nlocale = en-GB\n");
    assert!(ui.is_drawn("general-locale-note"));

    ui.click("general-locale-select");
    ui.click("general-locale-select-option-0");
    assert_eq!(ui.settings::<SettingsGeneralSnapshot>().locale, None);
    assert_eq!(ui.conf(), "[General]\n");
}

#[gpui::test]
fn under_an_environment_variable_a_locale_click_still_writes_and_the_override_is_drawn(
    app: &mut TestAppContext,
) {
    let var = "PERSONAL_LEDGER_GENERAL__LOCALE";
    let mut ui =
        Harness::with_configuration(app, &lib_config::Config::default(), &[(var, "en-GB")]);
    ui.press("g s enter");
    assert!(ui.is_drawn("general-locale-note"));

    ui.click("general-locale-select");
    ui.click("general-locale-select-option-3");

    assert_eq!(ui.conf(), "[General]\nlocale = en-US\n");
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().locale.as_deref(),
        Some("en-US")
    );
    let note = ui.settings::<SettingsGeneralSnapshot>().locale_note;
    assert!(note.is_some_and(|note| note.contains(var)));
}

#[gpui::test]
fn the_locale_data_folder_and_this_device_box_are_drawn(app: &mut TestAppContext) {
    let mut ui = on_general(app);

    assert!(ui.is_drawn("general-locale"));
    assert!(ui.is_drawn("general-data-folder"));
    assert!(ui.is_drawn("general-data-folder-change"));
    assert!(
        !ui.is_drawn("general-data-folder-note"),
        "nothing overrides it"
    );
    assert!(ui.is_drawn("general-this-device"));
    // A test Shell opened no Ledger file, and writes its own conf file.
    let general = ui.settings::<SettingsGeneralSnapshot>();
    assert_eq!(general.last_ledger, None);
    assert_eq!(
        general.configuration,
        Some(ui.conf_path().display().to_string())
    );
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

#[gpui::test]
fn clicking_test_raises_the_four_test_toasts(app: &mut TestAppContext) {
    let mut ui = on_general(app);
    assert!(ui.is_drawn("general-toasts-test"));

    ui.click("general-toasts-test");
    ui.advance_clock(Duration::from_millis(3500));

    let kinds: Vec<String> = ui
        .toasts()
        .history
        .into_iter()
        .map(|toast| toast.kind)
        .collect();
    assert_eq!(kinds, ["Error", "Warning", "Info", "Success"]);
    assert!(ui.settings::<SettingsGeneralSnapshot>().toasts_on);
}

#[gpui::test]
fn change_opens_the_folder_picker_and_use_writes_the_data_folder(app: &mut TestAppContext) {
    let dir = common::data_folder();
    let mut ui = on_general(app);
    ui.browse_from(dir.path());

    ui.click("general-data-folder-change");
    let picker = ui.explorer().expect("the picker is open");
    assert_eq!(picker.mode, lib_shell::ExplorerMode::PickFolder);
    assert_eq!(picker.folder, dir.path());
    assert!(ui.is_drawn("explorer-current"), "the header names it");
    ui.click("explorer-row-1");
    ui.click("explorer-open");

    let finance = dir.path().join("finance");
    assert_eq!(ui.explorer(), None);
    assert_eq!(ui.settings_frame().page, "General");
    assert_eq!(
        ui.conf(),
        format!("[General]\ndata = {}\n", finance.display())
    );
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().data_folder_shown,
        finance.display().to_string()
    );
    assert_eq!(
        ui.toasts().visible.pop().map(|toast| toast.text),
        Some(format!("Data folder is now {}", finance.display()))
    );
}
