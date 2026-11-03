//! Settings › General driven by keys: focus lands on On start, `j`/`k` walk On start and Toasts,
//! and `h`/`l` change the one focused. Each change is saved to the conf file (ADR-0053) and Toasts
//! reach the Shell's Toasts; a key an environment variable sets is shown as such and left alone.
//! State is read back from `Root` and the conf file.

use super::common;
use bin_desktop::shell::router::SettingsGeneralSnapshot;

use common::Harness;
use gpui::TestAppContext;
use lib_core::OnStart;

/// Settings on General, the first page, with focus stepped into it.
fn in_general(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    assert_eq!(ui.settings_frame().page, "General");
    ui.press("l");
    assert!(ui.settings_frame().page_focused);
    ui
}

#[gpui::test]
fn focus_starts_on_on_start(app: &mut TestAppContext) {
    let mut ui = in_general(app);

    let general = ui.settings::<SettingsGeneralSnapshot>();
    assert_eq!(general.field, Some(0));
    assert_eq!(general.on_start, OnStart::ReopenLastLedger);
    assert!(general.toasts_on);
}

#[gpui::test]
fn j_and_k_walk_the_two_controls_and_stop_at_the_ends(app: &mut TestAppContext) {
    let mut ui = in_general(app);

    ui.press("j j");
    assert_eq!(ui.settings::<SettingsGeneralSnapshot>().field, Some(1));
    ui.press("k k");
    assert_eq!(ui.settings::<SettingsGeneralSnapshot>().field, Some(0));
}

#[gpui::test]
fn l_and_h_step_on_start_and_each_step_is_saved_to_the_conf_file(app: &mut TestAppContext) {
    let mut ui = in_general(app);

    ui.press("l");
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().on_start,
        OnStart::NoLedger
    );
    assert_eq!(ui.conf(), "[Personal-Ledger]\non_start = none\n");
    ui.press("l");
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().on_start,
        OnStart::NoLedger
    );
    ui.press("h");
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().on_start,
        OnStart::ReopenLastLedger
    );
    assert_eq!(ui.conf(), "[Personal-Ledger]\non_start = reopen\n");
}

#[gpui::test]
fn l_turns_toasts_off_and_h_turns_them_back_on(app: &mut TestAppContext) {
    let mut ui = in_general(app);

    ui.press("j l");
    assert!(!ui.settings::<SettingsGeneralSnapshot>().toasts_on);
    assert!(!ui.toasts().on);
    assert_eq!(ui.conf(), "[Personal-Ledger]\ntoasts = false\n");
    ui.press("h");
    assert!(ui.settings::<SettingsGeneralSnapshot>().toasts_on);
    assert!(ui.toasts().on);
}

#[gpui::test]
fn the_toasts_command_moves_the_general_control(app: &mut TestAppContext) {
    let mut ui = in_general(app);

    ui.press(": t o a s t s space o f f enter");
    assert!(!ui.toasts().on);
    assert!(!ui.settings::<SettingsGeneralSnapshot>().toasts_on);
    assert_eq!(ui.conf(), "[Personal-Ledger]\ntoasts = false\n");
}

#[gpui::test]
fn a_setting_an_environment_variable_sets_is_shown_and_left_alone(app: &mut TestAppContext) {
    let var = "PERSONAL_LEDGER_PERSONAL_LEDGER__ON_START";
    let mut ui = Harness::with_configuration(
        app,
        &lib_config::PersonalLedgerConfig::default(),
        &[(var, "reopen")],
    );
    ui.press("g s l");
    assert!(ui.is_drawn("shadowed-on_start"));
    assert!(!ui.is_drawn("shadowed-toasts"));

    ui.press("l");

    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().on_start,
        OnStart::ReopenLastLedger
    );
    assert_eq!(ui.conf(), "", "nothing is written under the variable");
    let status = ui.read(bin_desktop::shell::router::Root::status_message);
    assert!(status.is_some_and(|text| text.contains(var)));
}

#[gpui::test]
fn general_starts_on_the_on_start_and_toasts_the_conf_file_holds(app: &mut TestAppContext) {
    let mut config = lib_config::PersonalLedgerConfig::default();
    config.on_start = OnStart::NoLedger;
    config.toasts = false;
    let mut ui = Harness::with_configuration(app, &config, &[]);
    ui.press("g s l");

    let general = ui.settings::<SettingsGeneralSnapshot>();
    assert_eq!(general.on_start, OnStart::NoLedger);
    assert!(!general.toasts_on);
    assert!(!ui.toasts().on);
}

#[gpui::test]
fn escape_steps_back_to_the_index_and_off_the_controls(app: &mut TestAppContext) {
    let mut ui = in_general(app);

    ui.press("escape");
    assert!(!ui.settings_frame().page_focused);
    assert_eq!(ui.settings::<SettingsGeneralSnapshot>().field, None);
}

#[gpui::test]
fn the_status_line_matches_the_cheat_sheet(app: &mut TestAppContext) {
    let mut ui = in_general(app);

    ui.assert_cheat_sheet_matches_status("Settings keys");
}
