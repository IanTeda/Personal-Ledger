//! Settings › General driven by keys: focus lands on On start, `j`/`k` walk On start, Start view,
//! Toasts, the Locale and the Data folder, and `h`/`l` change the one focused; `enter` opens Start
//! view's and the Locale's dropdowns, whose `j`/`k` move, `enter` picks and `esc` closes (#1156);
//! `enter` on the Data folder opens the folder picker (DESKTOP-187), whose `ctrl-enter` writes it
//! and raises a Toast (DESKTOP-188), and `esc` changes nothing. Each change is saved to the conf file
//! (ADR-0053) and Toasts reach the Shell's Toasts; `t` on Toasts raises the Toasts test; a key an
//! environment variable sets is shown as such and left alone, but for the Locale, which is written
//! all the same (DESKTOP-186). A chosen Locale applies at the next start, so a note says so.
//! State is read back from `Root` and the conf file.

use super::common;
use bin_desktop::shell::router::SettingsGeneralSnapshot;

use bin_desktop::WindowSize;
use common::Harness;
use gpui::TestAppContext;
use lib_core::{OnStart, StartView};
use std::time::Duration;

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
fn j_and_k_walk_the_five_controls_and_stop_at_the_ends(app: &mut TestAppContext) {
    let mut ui = in_general(app);

    ui.press("j");
    assert_eq!(ui.settings::<SettingsGeneralSnapshot>().field, Some(1));
    ui.press("j");
    assert_eq!(ui.settings::<SettingsGeneralSnapshot>().field, Some(2));
    ui.press("j");
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().field,
        Some(3),
        "the Locale comes after Toasts"
    );
    ui.press("j j");
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().field,
        Some(4),
        "the Data folder comes after the Locale, and is the last"
    );
    ui.press("k k k k k");
    assert_eq!(ui.settings::<SettingsGeneralSnapshot>().field, Some(0));
}

#[gpui::test]
fn l_and_h_step_the_locale_and_each_step_is_saved_with_a_note_till_the_next_start(
    app: &mut TestAppContext,
) {
    let mut ui = in_general(app);
    ui.press("j j j");
    let general = ui.settings::<SettingsGeneralSnapshot>();
    assert_eq!(general.locale, None, "System, with no key in the file");
    assert_eq!(general.locale_note, None);
    let toasts = ui.toasts().history.len();

    ui.press("l");

    let general = ui.settings::<SettingsGeneralSnapshot>();
    assert_eq!(general.locale.as_deref(), Some("en-AU"));
    assert_eq!(ui.conf(), "[General]\nlocale = en-AU\n");
    assert!(general.locale_note.is_some_and(|note| {
        note.starts_with("Saved. en-AU applies when Personal Ledger next starts")
    }));
    assert!(ui.is_drawn("general-locale-note"));
    assert_eq!(ui.toasts().history.len(), toasts, "no Toast");

    ui.press("l l l");
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().locale.as_deref(),
        Some("en-US"),
        "stops at the last choice"
    );
    assert_eq!(ui.conf(), "[General]\nlocale = en-US\n");

    ui.press("h h h");
    assert_eq!(ui.settings::<SettingsGeneralSnapshot>().locale, None);
    assert_eq!(ui.conf(), "[General]\n", "System takes the line out");
}

#[gpui::test]
fn an_unsupported_locale_in_the_file_shows_as_system_and_en_us_still_writes(
    app: &mut TestAppContext,
) {
    let mut ui = Harness::with_conf_file(app, "[General]\nlocale = fr-FR\n");
    ui.press("g s l j j j");
    assert_eq!(ui.settings::<SettingsGeneralSnapshot>().locale, None);

    ui.press("l l l");

    assert_eq!(ui.conf(), "[General]\nlocale = en-US\n");
}

#[gpui::test]
fn a_locale_an_environment_variable_sets_is_still_written_and_the_override_noted(
    app: &mut TestAppContext,
) {
    let var = "PERSONAL_LEDGER_GENERAL__LOCALE";
    let mut ui =
        Harness::with_configuration(app, &lib_config::Config::default(), &[(var, "en-GB")]);
    ui.press("g s l j j j");
    assert!(
        ui.settings::<SettingsGeneralSnapshot>()
            .locale_note
            .is_some_and(|note| note.starts_with(&format!("{var} overrides this for this run")))
    );

    ui.press("l");

    assert_eq!(ui.conf(), "[General]\nlocale = en-AU\n");
    assert!(ui.is_drawn("general-locale-note"));
    assert!(!ui.is_drawn("shadowed-locale"));
}

#[gpui::test]
fn l_and_h_step_on_start_and_each_step_is_saved_to_the_conf_file(app: &mut TestAppContext) {
    let mut ui = in_general(app);

    ui.press("l");
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().on_start,
        OnStart::NoLedger
    );
    assert_eq!(ui.conf(), "[General]\non_start = none\n");
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
    assert_eq!(ui.conf(), "[General]\non_start = reopen\n");
}

#[gpui::test]
fn l_and_h_step_start_view_through_its_choices_and_each_step_is_saved(app: &mut TestAppContext) {
    let mut ui = in_general(app);
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().start_view,
        "dashboard"
    );

    ui.press("j l");
    assert_eq!(ui.settings::<SettingsGeneralSnapshot>().start_view, "last");
    assert_eq!(ui.conf(), "[General]\nstart_view = last\n");
    ui.press("l l l l l l");
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().start_view,
        "budgets",
        "stops at the last choice"
    );
    assert_eq!(ui.conf(), "[General]\nstart_view = budgets\n");
    ui.press("h");
    assert_eq!(ui.settings::<SettingsGeneralSnapshot>().start_view, "bills");
    assert_eq!(ui.conf(), "[General]\nstart_view = bills\n");
}

#[gpui::test]
fn a_start_view_from_the_file_that_isnt_a_choice_is_named_and_steps_from_the_dashboard(
    app: &mut TestAppContext,
) {
    let mut config = lib_config::Config::default();
    config.general.start_view = StartView::Noun("units".into());
    let mut ui = Harness::with_configuration(app, &config, &[]);
    ui.press("g s l");
    assert_eq!(ui.settings::<SettingsGeneralSnapshot>().start_view, "units");
    assert!(ui.is_drawn("general-start-view-unlisted"));

    ui.press("j l");
    assert_eq!(ui.settings::<SettingsGeneralSnapshot>().start_view, "last");
}

#[gpui::test]
fn a_start_view_an_environment_variable_sets_is_shown_and_left_alone(app: &mut TestAppContext) {
    let var = "PERSONAL_LEDGER_GENERAL__START_VIEW";
    let mut ui =
        Harness::with_configuration(app, &lib_config::Config::default(), &[(var, "bills")]);
    ui.press("g s l");
    assert!(ui.is_drawn("shadowed-start_view"));

    ui.press("j l");

    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().start_view,
        "dashboard"
    );
    assert_eq!(ui.conf(), "", "nothing is written under the variable");
}

#[gpui::test]
fn l_turns_toasts_off_and_h_turns_them_back_on(app: &mut TestAppContext) {
    let mut ui = in_general(app);

    ui.press("j j l");
    assert!(!ui.settings::<SettingsGeneralSnapshot>().toasts_on);
    assert!(!ui.toasts().on);
    assert_eq!(ui.conf(), "[General]\ntoasts = false\n");
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
    assert_eq!(ui.conf(), "[General]\ntoasts = false\n");
}

#[gpui::test]
fn a_setting_an_environment_variable_sets_is_shown_and_left_alone(app: &mut TestAppContext) {
    let var = "PERSONAL_LEDGER_GENERAL__ON_START";
    let mut ui =
        Harness::with_configuration(app, &lib_config::Config::default(), &[(var, "reopen")]);
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
fn general_starts_on_the_on_start_start_view_and_toasts_the_conf_file_holds(
    app: &mut TestAppContext,
) {
    let mut config = lib_config::Config::default();
    config.general.on_start = OnStart::NoLedger;
    config.general.start_view = StartView::Last;
    config.general.toasts = false;
    let mut ui = Harness::with_configuration(app, &config, &[]);
    ui.press("g s l");

    let general = ui.settings::<SettingsGeneralSnapshot>();
    assert_eq!(general.on_start, OnStart::NoLedger);
    assert_eq!(general.start_view, "last");
    assert!(!general.toasts_on);
    assert!(!ui.toasts().on);
}

#[gpui::test]
fn enter_opens_start_views_dropdown_and_j_and_enter_pick_the_next_choice(app: &mut TestAppContext) {
    let mut ui = in_general(app);
    ui.press("j");
    assert_eq!(hint_keys(&mut ui), ["j/k", "h/l", "enter"]);

    ui.press("enter");
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().start_view_list,
        Some(0),
        "on the Dashboard"
    );
    assert!(ui.is_drawn("general-start-view-option-0"));
    assert_eq!(hint_keys(&mut ui), ["enter", "j/k", "esc"]);
    ui.press("j");
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().field,
        Some(1),
        "j moves the highlight, not the focus"
    );
    assert_eq!(ui.conf(), "", "nothing picked yet");

    ui.press("enter");

    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().start_view_list,
        None,
        "closed"
    );
    assert_eq!(ui.settings::<SettingsGeneralSnapshot>().start_view, "last");
    assert_eq!(ui.conf(), "[General]\nstart_view = last\n");
}

#[gpui::test]
fn escape_closes_an_open_dropdown_without_leaving_the_page(app: &mut TestAppContext) {
    let mut ui = in_general(app);
    ui.press("j enter j j");
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().start_view_list,
        Some(2)
    );

    ui.press("escape");

    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().start_view_list,
        None,
        "closed"
    );
    assert!(ui.settings_frame().page_focused, "still on the page");
    let general = ui.settings::<SettingsGeneralSnapshot>();
    assert_eq!(general.field, Some(1));
    assert_eq!(general.start_view, "dashboard");
    assert_eq!(ui.conf(), "");

    ui.press("escape");
    assert!(!ui.settings_frame().page_focused, "then back to the index");
}

#[gpui::test]
fn the_locale_dropdown_opens_picks_and_cancels(app: &mut TestAppContext) {
    let mut ui = in_general(app);
    ui.press("j j j enter");
    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().locale_list,
        Some(0),
        "on System"
    );
    assert!(ui.is_drawn("general-locale-select-option-0"));
    ui.press("j j escape");
    assert_eq!(ui.settings::<SettingsGeneralSnapshot>().locale_list, None);
    assert_eq!(ui.conf(), "", "cancelled");

    ui.press("enter k enter");

    assert_eq!(
        ui.settings::<SettingsGeneralSnapshot>().locale.as_deref(),
        Some("en-US"),
        "k wraps from System to the last"
    );
    assert_eq!(ui.conf(), "[General]\nlocale = en-US\n");
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

fn hint_keys(ui: &mut Harness<'_>) -> Vec<String> {
    ui.status()
        .map(|status| status.hints.into_iter().map(|(key, _)| key).collect())
        .unwrap_or_default()
}

#[gpui::test]
fn at_medium_the_hints_carry_one_esc(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Medium);
    ui.press("g s enter l");
    assert!(ui.settings_frame().page_focused);

    assert_eq!(hint_keys(&mut ui), ["j/k", "h/l", "[/]", "p", "esc"]);
}

#[gpui::test]
fn at_large_the_hints_are_field_and_change(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Large);
    ui.press("g s l");
    assert!(ui.settings_frame().page_focused);

    assert_eq!(hint_keys(&mut ui), ["j/k", "h/l"]);
}

#[gpui::test]
fn after_close_this_devices_last_ledger_reads_none(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press(": c l o s e enter");
    ui.press("g s");

    assert_eq!(ui.settings_frame().page, "General");
    assert_eq!(ui.settings::<SettingsGeneralSnapshot>().last_ledger, None);
}

fn visible_kinds(ui: &mut Harness<'_>) -> Vec<String> {
    ui.toasts()
        .visible
        .into_iter()
        .map(|toast| toast.kind)
        .collect()
}

#[gpui::test]
fn t_on_toasts_raises_the_four_test_toasts_as_the_clock_runs(app: &mut TestAppContext) {
    let mut ui = in_general(app);
    ui.press("t");
    assert!(ui.toasts().visible.is_empty(), "t on On start does nothing");

    ui.press("j j t");
    assert_eq!(visible_kinds(&mut ui), ["Success"]);
    ui.advance_clock(Duration::from_millis(1500));
    assert_eq!(visible_kinds(&mut ui), ["Success", "Info"]);
    ui.advance_clock(Duration::from_secs(2));
    assert_eq!(visible_kinds(&mut ui), ["Info", "Warning", "Error"]);
    assert_eq!(ui.toasts().history.len(), 4);
    assert!(ui.settings::<SettingsGeneralSnapshot>().toasts_on);
    assert_eq!(ui.conf(), "", "the test writes no setting");
}

#[gpui::test]
fn on_toasts_the_hints_read_on_off_and_test(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Large);
    ui.press("g s l j j");

    let hints = ui.status().map(|status| status.hints).unwrap_or_default();
    assert_eq!(
        hints,
        [
            ("j/k".to_string(), "field".to_string()),
            ("h/l".to_string(), "on / off".to_string()),
            ("t".to_string(), "test".to_string()),
        ]
    );
}

/// Steps Settings' index with `j` until `page` is the one selected, then into it.
fn on_settings_page(ui: &mut Harness<'_>, page: &str) {
    ui.press("g s");
    for _ in 0..20 {
        if ui.settings_frame().page == page {
            break;
        }
        ui.press("j");
    }
    assert_eq!(ui.settings_frame().page, page);
    ui.press("l");
}

/// A THIS DEVICE page names the configuration file at every width.
fn names_the_configuration_file(app: &mut TestAppContext, page: &str) {
    let mut ui = Harness::new(app);
    #[expect(clippy::expect_used, reason = "a test with no home folder can't run")]
    let home = dirs::home_dir().expect("a home folder");
    ui.set_configuration_file(Some(
        home.join(".config/personal-ledger/personal-ledger.conf"),
    ));
    on_settings_page(&mut ui, page);
    for size in [WindowSize::Large, WindowSize::XLarge] {
        ui.resize_to(size);
        assert_eq!(
            ui.status_right(size),
            "~/.config/personal-ledger/personal-ledger.conf",
            "{page}"
        );
    }
    ui.resize_to(WindowSize::Medium);
    assert_eq!(
        ui.status_right(WindowSize::Medium),
        "\u{2026}/personal-ledger.conf",
        "{page}"
    );
}

#[gpui::test]
fn general_names_the_configuration_file(app: &mut TestAppContext) {
    names_the_configuration_file(app, "General");
}

#[gpui::test]
fn display_names_the_configuration_file(app: &mut TestAppContext) {
    names_the_configuration_file(app, "Display");
}

#[gpui::test]
fn logs_names_the_configuration_file(app: &mut TestAppContext) {
    names_the_configuration_file(app, "Tracing");
}

#[gpui::test]
fn about_names_the_configuration_file(app: &mut TestAppContext) {
    names_the_configuration_file(app, "About");
}

#[gpui::test]
fn a_this_ledger_page_names_the_ledger_file(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.set_configuration_file(Some("/etc/pl/personal-ledger.conf".into()));
    on_settings_page(&mut ui, "Units");
    let right = ui.status_right(WindowSize::Large);
    assert!(!right.contains("personal-ledger.conf"), "{right}");
    assert!(right.ends_with(".pldb"), "{right}");
}

#[gpui::test]
fn with_no_ledger_open_general_still_names_the_configuration_file(app: &mut TestAppContext) {
    let mut ui = Harness::without_ledger(app);
    ui.set_configuration_file(Some("/etc/pl/personal-ledger.conf".into()));
    ui.press("g s");
    assert_eq!(ui.settings_frame().page, "General");
    ui.press("l");
    assert_eq!(
        ui.status_right(WindowSize::Large),
        "/etc/pl/personal-ledger.conf"
    );
}

#[gpui::test]
fn with_no_configuration_file_the_right_side_says_so(app: &mut TestAppContext) {
    let mut ui = in_general(app);
    ui.set_configuration_file(None);
    assert_eq!(ui.status_right(WindowSize::Large), "no configuration file");
}

/// General with the Data folder focused on `dir`.
fn on_the_data_folder<'a>(app: &'a mut TestAppContext, dir: &std::path::Path) -> Harness<'a> {
    let mut ui = in_general(app);
    ui.browse_from(dir);
    ui.press("j j j j");
    assert_eq!(ui.settings::<SettingsGeneralSnapshot>().field, Some(4));
    ui
}

#[gpui::test]
fn enter_on_the_data_folder_picks_one_and_returns_to_general_with_a_toast(
    app: &mut TestAppContext,
) {
    let dir = common::data_folder();
    let mut ui = on_the_data_folder(app, dir.path());

    ui.press("enter");
    let open = ui.explorer().expect("enter opens the picker");
    assert_eq!(open.mode, lib_shell::ExplorerMode::PickFolder);
    assert_eq!(open.folder, dir.path(), "it opens in the Data folder");
    ui.press("j l");
    ui.press("ctrl-enter");

    let finance = dir.path().join("finance");
    assert_eq!(ui.explorer(), None);
    assert_eq!(ui.settings_frame().page, "General");
    let general = ui.settings::<SettingsGeneralSnapshot>();
    assert_eq!(general.field, Some(4), "the Data folder keeps the keyboard");
    assert_eq!(general.data_folder, finance.display().to_string());
    assert_eq!(general.data_folder_shown, finance.display().to_string());
    assert_eq!(
        ui.conf(),
        format!("[General]\ndata = {}\n", finance.display())
    );
    let toast = ui.toasts().visible.pop().expect("a Success Toast");
    assert_eq!(toast.kind, "Success");
    assert_eq!(
        toast.text,
        format!("Data folder is now {}", finance.display())
    );
}

#[gpui::test]
fn escape_from_the_data_folder_picker_changes_nothing(app: &mut TestAppContext) {
    let dir = common::data_folder();
    let mut ui = on_the_data_folder(app, dir.path());

    ui.press("enter l escape");

    assert_eq!(ui.explorer(), None);
    assert_eq!(ui.conf(), "");
    let general = ui.settings::<SettingsGeneralSnapshot>();
    assert_eq!(general.field, Some(4));
    assert_eq!(general.data_folder, dir.path().display().to_string());
    assert!(ui.toasts().visible.is_empty());
}

#[gpui::test]
fn a_data_folder_an_environment_variable_sets_is_still_written_and_the_override_noted(
    app: &mut TestAppContext,
) {
    let dir = common::data_folder();
    let var = "PERSONAL_LEDGER_GENERAL__DATA";
    let mut ui = Harness::with_configuration(
        app,
        &lib_config::Config::default(),
        &[(var, &dir.path().display().to_string())],
    );
    ui.browse_from(dir.path());
    ui.press("g s l j j j j");
    let note = ui.settings::<SettingsGeneralSnapshot>().data_folder_note;
    assert!(
        note.is_some_and(|note| note.starts_with(&format!(
            "{var} sets the Data folder to {} for now",
            dir.path().display()
        ))),
        "the note names the override and the folder in effect"
    );
    assert!(ui.is_drawn("general-data-folder-note"));

    ui.press("enter j l ctrl-enter");

    let finance = dir.path().join("finance");
    assert_eq!(
        ui.conf(),
        format!("[General]\ndata = {}\n", finance.display()),
        "written for when the override is gone"
    );
    let general = ui.settings::<SettingsGeneralSnapshot>();
    assert_eq!(general.data_folder_shown, finance.display().to_string());
    assert_eq!(
        general.data_folder,
        dir.path().display().to_string(),
        "the override still decides"
    );
    assert!(ui.toasts().visible.is_empty(), "nothing changed in effect");
    ui.press(": o p e n enter");
    assert_eq!(
        ui.explorer().map(|explorer| explorer.folder),
        Some(dir.path().to_path_buf()),
        ":open still starts where the override says"
    );
}
