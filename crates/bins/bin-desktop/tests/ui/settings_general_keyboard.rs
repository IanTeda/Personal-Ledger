//! Settings › General driven by keys: focus lands on On start, `j`/`k` walk On start and Toasts,
//! and `h`/`l` change the one focused. On start is what the state file keeps at quit, and Toasts
//! reach the Shell's Toasts. State is read back from `Root`.

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
fn l_and_h_step_on_start_and_quit_keeps_it(app: &mut TestAppContext) {
    let mut ui = in_general(app);

    ui.press("l");
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().on_start,
        OnStart::NoLedger
    );
    assert_eq!(
        ui.read_app(|shell, cx| shell.client_preferences(cx).on_start),
        OnStart::NoLedger
    );
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
}

#[gpui::test]
fn l_turns_toasts_off_and_h_turns_them_back_on(app: &mut TestAppContext) {
    let mut ui = in_general(app);

    ui.press("j l");
    assert!(!ui.settings::<SettingsGeneralSnapshot>().toasts_on);
    assert!(!ui.toasts().on);
    assert!(!ui.read_app(|shell, cx| shell.client_preferences(cx).toasts_on));
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
