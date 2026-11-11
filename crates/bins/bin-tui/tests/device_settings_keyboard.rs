//! Keyboard tests for a device's settings as Configuration (ADR-0053, #981): Toasts, the Colour
//! Theme and Appearance, the date style and On start are saved to the user's
//! `personal-ledger.conf` as they change on Settings or by command, and a key an environment
//! variable sets is shown as such and left unsaved.

mod common;

use common::{press, read, run_command, screen, shell, writing_config};

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
