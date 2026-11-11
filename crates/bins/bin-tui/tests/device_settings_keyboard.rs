//! Keyboard tests for a device's settings as Configuration (ADR-0053, #981): Toasts, the Colour
//! Theme and Appearance, the date style and On start are saved to the user's
//! `personal-ledger.conf` as they change on Settings or by command, and a key an environment
//! variable sets is shown as such and left unsaved.

mod common;

use common::{press, read, run_command, screen, screen_at, shell, writing_config};

#[test]
fn toasts_off_by_command_is_saved_to_the_config_file() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let (mut shell, path) = writing_config(shell(), &dir, &[]);

    run_command(&mut shell, "toasts off");

    assert_eq!(read(&path), "[General]\ntoasts = false\n");
}

#[test]
fn the_date_style_row_saves_its_choice_to_the_config_file() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let (mut shell, path) = writing_config(shell(), &dir, &[]);
    run_command(&mut shell, "settings display");

    press(&mut shell, "j j Enter Left Enter");

    assert_eq!(read(&path), "[Display]\ndate_style = iso\n");
    assert!(
        screen(&mut shell).contains("date-style saved to"),
        "the status line names where it went"
    );
}

#[test]
fn the_colour_theme_popup_saves_on_enter_and_not_on_esc() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let (mut shell, path) = writing_config(shell(), &dir, &[]);
    run_command(&mut shell, "settings display");

    press(&mut shell, "j Enter j Esc");
    assert_eq!(read(&path), "", "esc reverts the preview and saves nothing");

    press(&mut shell, "Enter j Enter");
    let text = read(&path);
    assert!(text.starts_with("[Display]\ncolour_theme = "), "{text}");
    assert!(
        screen(&mut shell).contains("colour-theme saved to"),
        "the status line names where it went"
    );
}

#[test]
fn set_appearance_and_set_on_start_save_to_the_config_file() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let (mut shell, path) = writing_config(shell(), &dir, &[]);

    run_command(&mut shell, "set appearance light");
    run_command(&mut shell, "set on-start none");

    assert_eq!(
        read(&path),
        "[Display]\ncolour_appearance = light\n\n[General]\non_start = none\n"
    );
}

#[test]
fn a_device_setting_an_environment_variable_sets_is_shown_and_not_saved() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let (mut shell, path) = writing_config(
        shell(),
        &dir,
        &[("PERSONAL_LEDGER_GENERAL__ON_START", "reopen")],
    );
    run_command(&mut shell, "settings general");
    let frame = screen(&mut shell);
    assert!(
        frame.contains("set by PERSONAL_LEDGER_GENERAL__"),
        "the row names what outranks it:\n{frame}"
    );

    press(&mut shell, "Enter");
    run_command(&mut shell, "set on-start none");

    assert_eq!(read(&path), "", "nothing is written under the variable");
    assert!(
        screen(&mut shell).contains("PERSONAL_LEDGER_GENERAL__ON_START"),
        "the status line says why"
    );
    assert!(!screen(&mut shell).contains("start with no ledger open"));
}

#[test]
fn enter_on_an_unmoved_colour_popup_saves_nothing_and_keeps_an_unknown_theme() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("personal-ledger.conf");
    std::fs::write(&path, "[Display]\ncolour_theme = from_a_newer_release\n").expect("write");
    let mut config = lib_config::Config::default();
    config.display.colour_theme = Some("from_a_newer_release".into());
    let writer = lib_config::ConfigWriter::new(Some(path.clone()), Vec::new(), []);
    let mut shell = shell().with_configuration(&config, writer);
    run_command(&mut shell, "settings display");

    press(&mut shell, "j Enter Enter");

    assert_eq!(
        read(&path),
        "[Display]\ncolour_theme = from_a_newer_release\n",
        "an id this release doesn't know survives a look at the list"
    );
}

#[test]
fn the_start_view_row_saves_its_choice_to_the_config_file() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let (mut shell, path) = writing_config(shell(), &dir, &[]);
    run_command(&mut shell, "settings general");

    press(&mut shell, "j Enter Right Enter");

    assert_eq!(read(&path), "[General]\nstart_view = last\n");
    let frame = screen(&mut shell);
    assert!(frame.contains("start-view saved to"), "{frame}");
    assert!(
        frame.contains("last view"),
        "the row shows the choice:\n{frame}"
    );
}

#[test]
fn the_start_view_choice_keeps_the_chosen_noun_in_view() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let (mut shell, _) = writing_config(shell(), &dir, &[]);
    run_command(&mut shell, "settings general");

    // `←` from the Dashboard wraps to the last noun, past the row's right edge.
    press(&mut shell, "j Enter Left");

    let frame = screen(&mut shell);
    assert!(frame.contains(" tags "), "{frame}");
}

#[test]
fn set_start_view_saves_a_noun_and_refuses_one_the_tui_does_not_have() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let (mut shell, path) = writing_config(shell(), &dir, &[]);

    run_command(&mut shell, "set start-view bills");
    assert_eq!(
        read(&path),
        "",
        "nothing is written for a noun the TUI lacks"
    );

    run_command(&mut shell, "set start-view Units");
    assert_eq!(read(&path), "[General]\nstart_view = units\n");
}

#[test]
fn a_start_view_an_environment_variable_sets_is_shown_and_not_saved() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let (mut shell, path) = writing_config(
        shell(),
        &dir,
        &[("PERSONAL_LEDGER_GENERAL__START_VIEW", "settings")],
    );
    run_command(&mut shell, "settings general");
    assert!(
        screen(&mut shell).contains("set by PERSONAL_LEDGER_GENERAL__"),
        "the row names what outranks it"
    );

    run_command(&mut shell, "set start-view units");

    assert_eq!(read(&path), "", "nothing is written under the variable");
    assert!(
        screen(&mut shell).contains("PERSONAL_LEDGER_GENERAL__START_VIEW"),
        "the status line says why"
    );
}

/// Lets `seconds` pass, a tick at a time.
fn pass(shell: &mut bin_tui::Shell, seconds: u64) {
    let ticks = seconds * 1000 / 250;
    for _ in 0..ticks {
        shell.handle_event(bin_tui::event::Event::Tick);
    }
}

#[test]
fn t_on_toasts_raises_the_four_test_toasts_as_ticks_pass() {
    let mut shell = shell();
    run_command(&mut shell, "settings general");
    assert!(
        !screen(&mut shell).contains("t test"),
        "t test is not a hint off Toasts"
    );
    press(&mut shell, "j j");
    let frame = screen(&mut shell);
    assert!(frame.contains("t test"), "{frame}");
    assert!(frame.contains("[ test ]"), "{frame}");

    press(&mut shell, "t");
    assert!(screen(&mut shell).contains("Test: a Success Toast"));
    pass(&mut shell, 1);
    assert!(screen(&mut shell).contains("Test: an Info Toast"));
    pass(&mut shell, 1);
    assert!(screen(&mut shell).contains("Test: a Warning Toast"));
    pass(&mut shell, 1);
    let frame = screen(&mut shell);
    assert!(frame.contains("Test: an Error Toast"), "{frame}");
}

#[test]
fn toasts_test_by_command_raises_four_and_with_toasts_off_only_the_error_stacks() {
    let mut shell = shell();
    run_command(&mut shell, "toasts test");
    pass(&mut shell, 3);
    let frame = screen(&mut shell);
    assert!(frame.contains("Test: an Error Toast"), "{frame}");
    assert!(frame.contains("Test: a Warning Toast"), "{frame}");

    let mut shell = common::shell();
    run_command(&mut shell, "toasts off");
    run_command(&mut shell, "toasts test");
    pass(&mut shell, 3);
    let frame = screen(&mut shell);
    assert!(frame.contains("Test: an Error Toast"), "{frame}");
    assert!(!frame.contains("Test: a Success Toast"), "{frame}");
}

#[test]
fn the_locale_row_saves_its_choice_and_system_removes_the_key() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let (mut shell, path) = writing_config(shell(), &dir, &[]);
    run_command(&mut shell, "settings general");

    // system, en-US, en-GB, en-AU: the row opens on system, so `Right` twice is en-GB.
    press(&mut shell, "j j j Enter Right Right Enter");

    assert_eq!(read(&path), "[General]\nlocale = en-GB\n");
    let frame = screen_at(&mut shell, 96);
    assert!(
        frame.contains(
            "Saved. en-GB applies when Personal Ledger next starts; until then it reads en-US"
        ),
        "{frame}"
    );

    press(&mut shell, "Enter Left Left Enter");
    assert!(!read(&path).contains("locale"), "{}", read(&path));
}

#[test]
fn the_locale_choices_never_offer_the_pseudo_locale() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let (mut shell, _path) = writing_config(shell(), &dir, &[]);
    run_command(&mut shell, "settings general");

    press(&mut shell, "j j j Enter");
    let frame = screen_at(&mut shell, 96);

    for tag in ["system (now", "en-AU", "en-GB", "en-US"] {
        assert!(frame.contains(tag), "{tag} missing:\n{frame}");
    }
    assert!(!frame.contains("en-XA"), "{frame}");
}

#[test]
fn an_overridden_locale_still_writes_the_file_and_names_the_override() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let (mut shell, path) = writing_config(
        shell(),
        &dir,
        &[("PERSONAL_LEDGER_GENERAL__LOCALE", "en-AU")],
    );
    run_command(&mut shell, "settings general");

    press(&mut shell, "j j j Enter Right Right Enter");

    assert_eq!(read(&path), "[General]\nlocale = en-GB\n");
    let frame = screen_at(&mut shell, 96);
    assert!(frame.contains("PERSONAL_LEDGER_GENERAL__LOCALE"), "{frame}");
    assert!(frame.contains("in effect"), "{frame}");
}

/// A configuration file whose `data` is a folder holding `finance/` and `old/` and a stray file,
/// and the shell writing to it.
#[expect(
    clippy::expect_used,
    reason = "test support: a failed set-up should fail the test loudly"
)]
fn data_folder_shell(
    shell: bin_tui::Shell,
    env: &[(&str, &str)],
) -> (
    bin_tui::Shell,
    std::path::PathBuf,
    tempfile::TempDir,
    tempfile::TempDir,
) {
    let config_dir = tempfile::tempdir().expect("a temporary directory");
    let data = tempfile::tempdir().expect("a temporary directory");
    std::fs::create_dir(data.path().join("finance")).expect("a folder");
    std::fs::create_dir(data.path().join("old")).expect("a folder");
    std::fs::write(data.path().join("household.pldb"), b"x").expect("a file");
    let path = config_dir.path().join("personal-ledger.conf");
    std::fs::write(
        &path,
        format!("[General]\ndata = {}\n", data.path().display()),
    )
    .expect("write");
    let writer = lib_config::ConfigWriter::new(
        Some(path.clone()),
        Vec::new(),
        env.iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string())),
    );
    let shell = shell.with_configuration(&lib_config::Config::default(), writer);
    (shell, path, config_dir, data)
}

#[test]
fn enter_on_the_data_folder_opens_the_picker_and_u_writes_the_folder_it_shows() {
    let (mut shell, path, _config, data) = data_folder_shell(shell(), &[]);
    run_command(&mut shell, "settings general");
    let frame = screen(&mut shell);
    assert!(frame.contains("data folder"), "{frame}");

    press(&mut shell, "j j j j Enter");
    let frame = screen(&mut shell);
    assert!(frame.contains("Choose data folder"), "{frame}");
    assert!(
        frame.contains("now "),
        "the header names the folder in use:\n{frame}"
    );
    assert!(
        frame.contains("finance/") && frame.contains("old/"),
        "{frame}"
    );
    assert!(
        frame.contains("household.pldb"),
        "files are listed:\n{frame}"
    );
    assert!(frame.contains("[ use "), "{frame}");

    press(&mut shell, "l");
    assert!(
        screen(&mut shell).contains("[ use finance ]"),
        "the chip names the folder shown"
    );
    press(&mut shell, "u");

    assert_eq!(
        read(&path),
        format!(
            "[General]\ndata = {}\n",
            data.path().join("finance").display()
        )
    );
    let frame = screen(&mut shell);
    assert!(!frame.contains("Choose data folder"), "{frame}");
    assert!(
        frame.contains("Settings › General"),
        "back on General:\n{frame}"
    );
    assert!(
        frame.contains("Data folder is now"),
        "the Success Toast:\n{frame}"
    );
}

#[test]
fn esc_in_the_picker_changes_nothing_and_enter_goes_into_the_folder_under_the_cursor() {
    let (mut shell, path, _config, data) = data_folder_shell(shell(), &[]);
    let before = read(&path);
    run_command(&mut shell, "settings general");
    press(&mut shell, "j j j j Enter");

    press(&mut shell, "j");
    press(&mut shell, "Enter");
    assert!(screen(&mut shell).contains("[ use old ]"));
    press(&mut shell, "h");
    assert!(screen(&mut shell).contains(&format!(
        "[ use {} ]",
        data.path().file_name().unwrap().to_string_lossy()
    )));
    press(&mut shell, "Esc");

    let frame = screen(&mut shell);
    assert!(!frame.contains("Choose data folder"), "{frame}");
    assert!(frame.contains("Settings › General"), "{frame}");
    assert_eq!(read(&path), before);
    assert!(!frame.contains("Data folder is now"), "{frame}");
}

#[test]
fn the_picker_fits_at_96_columns_and_the_mode_stays_normal() {
    let (mut shell, _path, _config, _data) = data_folder_shell(shell(), &[]);
    run_command(&mut shell, "settings general");
    press(&mut shell, "j j j j Enter");

    let frame = screen_at(&mut shell, 96);
    assert!(frame.contains("Choose data folder"), "{frame}");
    assert!(frame.contains("NORMAL"), "{frame}");
    for line in frame.lines() {
        assert!(line.chars().count() <= 96, "{line}");
    }
}

#[test]
fn an_override_names_itself_and_the_picker_still_writes_the_file() {
    let (mut shell, path, _config, data) = data_folder_shell(
        shell(),
        &[("PERSONAL_LEDGER_GENERAL__DATA", "/srv/elsewhere")],
    );
    run_command(&mut shell, "settings general");
    let frame = screen(&mut shell);
    assert!(
        frame.contains("set by PERSONAL_LEDGER_GENERAL__DATA"),
        "the row names what outranks it:\n{frame}"
    );

    press(&mut shell, "j j j j Enter l u");

    assert_eq!(
        read(&path),
        format!(
            "[General]\ndata = {}\n",
            data.path().join("finance").display()
        )
    );
}

#[test]
fn data_opens_the_picker_over_the_dashboard_with_or_without_a_ledger_and_returns_to_it() {
    for ledger_open in [true, false] {
        let base = if ledger_open {
            shell()
        } else {
            common::cold_shell(None)
        };
        let (mut shell, path, _config, _data) = data_folder_shell(base, &[]);
        let before = read(&path);
        let dashboard = screen(&mut shell);

        run_command(&mut shell, "data");
        let frame = screen(&mut shell);
        assert!(frame.contains("Choose data folder"), "{frame}");
        assert!(frame.contains("NORMAL"), "{frame}");

        press(&mut shell, "Esc");
        assert_eq!(screen(&mut shell), dashboard, "ledger open: {ledger_open}");
        assert_eq!(read(&path), before);
    }
}

#[test]
fn picking_the_current_folder_writes_nothing() {
    let (mut shell, path, _config, _data) = data_folder_shell(shell(), &[]);
    let before = read(&path);

    run_command(&mut shell, "data");
    press(&mut shell, "u");

    let frame = screen(&mut shell);
    assert!(!frame.contains("Choose data folder"), "{frame}");
    assert!(!frame.contains("Data folder is now"), "{frame}");
    assert_eq!(read(&path), before);
}

#[test]
fn data_takes_no_path_and_set_data_refuses_one() {
    let (mut shell, path, _config, _data) = data_folder_shell(shell(), &[]);
    let before = read(&path);

    run_command(&mut shell, "set data /tmp");

    assert_eq!(read(&path), before);
    assert!(!screen(&mut shell).contains("Choose data folder"));
}
