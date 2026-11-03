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

    assert_eq!(read(&path), "[Personal-Ledger]\ntoasts = false\n");
}

#[test]
fn the_date_style_row_saves_its_choice_to_the_config_file() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let (mut shell, path) = writing_config(shell(), &dir, &[]);
    run_command(&mut shell, "settings display");

    press(&mut shell, "j j Enter Left Enter");

    assert_eq!(read(&path), "[Personal-Ledger]\ndate_style = iso\n");
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
    assert!(
        text.starts_with("[Personal-Ledger]\ncolour_theme = "),
        "{text}"
    );
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
        "[Personal-Ledger]\ncolour_appearance = light\non_start = none\n"
    );
}

#[test]
fn a_device_setting_an_environment_variable_sets_is_shown_and_not_saved() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let (mut shell, path) = writing_config(
        shell(),
        &dir,
        &[("PERSONAL_LEDGER_PERSONAL_LEDGER__ON_START", "reopen")],
    );
    run_command(&mut shell, "settings general");
    let frame = screen(&mut shell);
    assert!(
        frame.contains("set by PERSONAL_LEDGER_PERSONAL_LEDGER__"),
        "the row names what outranks it:\n{frame}"
    );

    press(&mut shell, "Enter");
    run_command(&mut shell, "set on-start none");

    assert_eq!(read(&path), "", "nothing is written under the variable");
    assert!(
        screen(&mut shell).contains("PERSONAL_LEDGER_PERSONAL_LEDGER__ON_START"),
        "the status line says why"
    );
    assert!(!screen(&mut shell).contains("start with no ledger open"));
}

#[test]
fn enter_on_an_unmoved_colour_popup_saves_nothing_and_keeps_an_unknown_theme() {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("personal-ledger.conf");
    std::fs::write(
        &path,
        "[Personal-Ledger]\ncolour_theme = from_a_newer_release\n",
    )
    .expect("write");
    let mut config = lib_config::PersonalLedgerConfig::default();
    config.colour_theme = Some("from_a_newer_release".into());
    let writer = lib_config::ConfigWriter::new(Some(path.clone()), Vec::new(), []);
    let mut shell = shell().with_configuration(&config, writer);
    run_command(&mut shell, "settings display");

    press(&mut shell, "j Enter Enter");

    assert_eq!(
        read(&path),
        "[Personal-Ledger]\ncolour_theme = from_a_newer_release\n",
        "an id this release doesn't know survives a look at the list"
    );
}
