//! Keyboard tests for the Ledger file commands (#871, #910): the Shell starts by On start, and
//! `:open`, `:new` and `:close` act on a real Ledger file in a throwaway directory.

mod common;

use common::{cold_shell, press, run_command, screen, shell_with_failures, type_text};
use lib_database::{Domain, WriteFailure};

/// The top row of a drawn frame.
fn status_line(frame: &str) -> &str {
    frame.lines().next().unwrap_or_default()
}

/// The bottom row of a drawn frame.
fn hint_bar(frame: &str) -> &str {
    frame.lines().last().unwrap_or_default()
}

#[test]
fn cold_start_has_no_ledger_open_and_creates_no_file() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("household.pldb");
    let mut shell = cold_shell(Some(&path));

    let frame = screen(&mut shell);

    assert!(frame.contains("No Ledger open."), "{frame}");
    assert!(
        frame.contains("Use :new to create a Ledger or :open to open one."),
        "{frame}"
    );
    assert!(!path.exists(), "cold start created the Ledger file");
}

#[test]
fn with_no_ledger_open_the_status_line_names_only_the_product() {
    let mut shell = cold_shell(None);

    let frame = screen(&mut shell);

    assert_eq!(status_line(&frame).trim(), "Personal Ledger", "{frame}");
}

#[test]
fn with_no_ledger_open_the_hint_bar_is_the_only_row_under_the_view() {
    let mut shell = cold_shell(None);

    let frame = screen(&mut shell);
    let hints = hint_bar(&frame);

    assert!(
        hints.starts_with(" NORMAL \u{b7} : command \u{b7} ? help"),
        "{frame}"
    );
    assert!(hints.trim_end().ends_with("No Ledger open"), "{frame}");
    let above = frame.lines().rev().nth(1).unwrap_or_default();
    assert!(!above.contains('\u{2500}'), "a rule is drawn:\n{frame}");
}

#[test]
fn the_status_line_names_the_mode_when_it_is_not_normal() {
    let mut shell = cold_shell(None);

    press(&mut shell, ":");
    let frame = screen(&mut shell);

    assert_eq!(
        status_line(&frame).trim(),
        "Personal Ledger \u{b7} COMMAND",
        "{frame}"
    );
}

#[test]
fn open_with_a_path_opens_that_ledger_file() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("household.pldb");
    lib_database::open_storage(&path, chrono::Local::now().date_naive(), "Personal")
        .expect("a Ledger file to open");
    let mut shell = cold_shell(None);

    run_command(&mut shell, &format!("open {}", path.display()));
    let frame = screen(&mut shell);

    assert!(frame.contains("Opened ledger household.pldb"), "{frame}");
    assert!(!frame.contains("No Ledger open"), "{frame}");
    assert!(
        hint_bar(&frame).trim_end().ends_with("household.pldb"),
        "{frame}"
    );
    assert!(status_line(&frame).contains("Synced"), "{frame}");
}

#[test]
fn a_bare_open_opens_the_configured_ledger_file() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("configured.pldb");
    lib_database::open_storage(&path, chrono::Local::now().date_naive(), "Personal")
        .expect("a Ledger file to open");
    let mut shell = cold_shell(Some(&path));

    run_command(&mut shell, "open");
    let frame = screen(&mut shell);

    assert!(frame.contains("Opened ledger configured.pldb"), "{frame}");
}

#[test]
fn a_bare_open_with_no_file_configured_says_so_on_the_status_line() {
    let mut shell = cold_shell(None);

    run_command(&mut shell, "open");
    let frame = screen(&mut shell);

    assert!(
        hint_bar(&frame).contains("No Ledger file configured"),
        "{frame}"
    );
    assert!(frame.contains("No Ledger open."), "{frame}");
}

#[test]
fn opening_a_missing_file_raises_an_error_toast_and_creates_nothing() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("missing.pldb");
    let mut shell = cold_shell(None);

    run_command(&mut shell, &format!("open {}", path.display()));
    let frame = screen(&mut shell);

    assert!(
        frame.contains("\u{2717} Couldn't open ledger missing.pldb"),
        "{frame}"
    );
    assert!(frame.contains("No Ledger open."), "{frame}");
    assert!(!path.exists(), "a failed open created the file");
}

#[test]
fn opening_a_file_that_is_not_a_ledger_raises_an_error_toast() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("notes.pldb");
    std::fs::write(&path, "not a database").expect("a file to write");
    let mut shell = cold_shell(None);

    run_command(&mut shell, &format!("open {}", path.display()));
    let frame = screen(&mut shell);

    assert!(
        frame.contains("\u{2717} Couldn't open ledger notes.pldb"),
        "{frame}"
    );
    assert!(frame.contains("No Ledger open."), "{frame}");
}

#[test]
fn new_creates_a_ledger_file_and_opens_it() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("business.pldb");
    let mut shell = cold_shell(None);

    run_command(&mut shell, &format!("new {}", path.display()));
    let frame = screen(&mut shell);

    assert!(frame.contains("Created ledger business.pldb"), "{frame}");
    assert!(
        hint_bar(&frame).trim_end().ends_with("business.pldb"),
        "{frame}"
    );
    assert!(path.exists(), "no Ledger file was created");
}

#[test]
fn new_refuses_a_path_that_exists_and_writes_nothing() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("taken.pldb");
    std::fs::write(&path, "already here").expect("a file to write");
    let mut shell = cold_shell(None);

    run_command(&mut shell, &format!("new {}", path.display()));
    let frame = screen(&mut shell);

    assert!(hint_bar(&frame).contains("already exists"), "{frame}");
    assert!(frame.contains("No Ledger open."), "{frame}");
    assert_eq!(
        std::fs::read_to_string(&path).expect("the file is still there"),
        "already here"
    );
}

#[test]
fn close_returns_to_no_ledger_open() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("household.pldb");
    let mut shell = cold_shell(None);
    run_command(&mut shell, &format!("new {}", path.display()));

    run_command(&mut shell, "close");
    let frame = screen(&mut shell);

    assert!(frame.contains("No Ledger open."), "{frame}");
    assert!(
        hint_bar(&frame).trim_end().ends_with("No Ledger open"),
        "{frame}"
    );
    assert_eq!(status_line(&frame).trim(), "Personal Ledger", "{frame}");
}

/// A failed Units write, as the writer reports one for the last batch before the Ledger closes.
fn failed_units_write() -> WriteFailure {
    WriteFailure {
        domains: vec![Domain::Units],
        error: lib_database::Error::WriterStopped,
    }
}

#[test]
fn a_write_that_fails_just_before_close_raises_an_error_toast() {
    let (mut shell, report) = shell_with_failures();
    report
        .send(failed_units_write())
        .expect("the Shell holds the receiver");

    run_command(&mut shell, "close");
    let frame = screen(&mut shell);

    assert!(frame.contains("No Ledger open."), "{frame}");
    assert!(
        frame.contains("\u{2717} Last change to test.pldb wasn't saved"),
        "{frame}"
    );
}

#[test]
fn a_write_that_fails_just_before_open_raises_an_error_toast() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let other = directory.path().join("other.pldb");
    ledger_file(&other);
    let (mut shell, report) = shell_with_failures();
    report
        .send(failed_units_write())
        .expect("the Shell holds the receiver");

    run_command(&mut shell, &format!("open {}", other.display()));
    let frame = screen(&mut shell);

    assert!(frame.contains("Opened ledger other.pldb"), "{frame}");
    assert!(
        frame.contains("Last change to test.pldb wasn't saved"),
        "{frame}"
    );
}

#[test]
fn a_unit_added_after_new_is_there_after_close_and_open() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("household.pldb");
    let mut shell = cold_shell(None);
    run_command(&mut shell, &format!("new {}", path.display()));
    add_gold(&mut shell);
    run_command(&mut shell, "close");

    run_command(&mut shell, &format!("open {}", path.display()));
    press(&mut shell, "g u");
    let frame = screen(&mut shell);

    assert!(frame.contains("xau"), "{frame}");
    assert!(
        frame.contains("aud"),
        "the seed isn't written twice:\n{frame}"
    );
}

#[test]
fn reopening_the_open_ledger_file_keeps_its_last_write() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("household.pldb");
    let mut shell = cold_shell(None);
    run_command(&mut shell, &format!("new {}", path.display()));
    add_gold(&mut shell);

    run_command(&mut shell, &format!("open {}", path.display()));
    press(&mut shell, "g u");
    let frame = screen(&mut shell);

    assert!(frame.contains("xau"), "{frame}");
}

/// Adds the Unit `xau` (Gold) in the Units View.
fn add_gold(shell: &mut bin_tui::Shell) {
    press(shell, "g u n");
    type_text(shell, "xau");
    press(shell, "Tab Tab");
    type_text(shell, "Gold");
    press(shell, "Enter");
}

/// A Ledger file at `path`, created and migrated as `:new` would.
fn ledger_file(path: &std::path::Path) {
    assert!(
        lib_database::open_storage(path, chrono::Local::now().date_naive(), "Personal").is_ok(),
        "a Ledger file to open"
    );
}

/// On start set to start with no Ledger open.
fn no_ledger_on_start() -> lib_core::ClientPreferences {
    lib_core::ClientPreferences {
        on_start: lib_core::OnStart::NoLedger,
        ..lib_core::ClientPreferences::default()
    }
}

#[test]
fn reopen_last_ledger_opens_the_remembered_ledger_file_at_start() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("household.pldb");
    ledger_file(&path);
    let mut shell = cold_shell(None);

    shell.open_on_start(Some(&path));
    let frame = screen(&mut shell);

    assert!(frame.contains("Opened ledger household.pldb"), "{frame}");
    assert!(
        hint_bar(&frame).trim_end().ends_with("household.pldb"),
        "{frame}"
    );
}

#[test]
fn start_with_no_ledger_open_ignores_the_remembered_ledger_file() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("household.pldb");
    ledger_file(&path);
    let mut shell = cold_shell(None).with_client_preferences(&no_ledger_on_start());

    shell.open_on_start(Some(&path));
    let frame = screen(&mut shell);

    assert!(frame.contains("No Ledger open."), "{frame}");
}

#[test]
fn an_explicit_file_opens_at_start_whatever_on_start_says() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let explicit = directory.path().join("explicit.pldb");
    let remembered = directory.path().join("remembered.pldb");
    ledger_file(&explicit);
    ledger_file(&remembered);
    let mut shell = cold_shell(Some(&explicit)).with_client_preferences(&no_ledger_on_start());

    shell.open_on_start(Some(&remembered));
    let frame = screen(&mut shell);

    assert!(
        hint_bar(&frame).trim_end().ends_with("explicit.pldb"),
        "{frame}"
    );
}

#[test]
fn a_remembered_file_that_won_t_open_raises_an_error_toast_and_starts_with_none_open() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("gone.pldb");
    let mut shell = cold_shell(None);

    shell.open_on_start(Some(&path));
    let frame = screen(&mut shell);

    assert!(
        frame.contains("\u{2717} Couldn't open ledger gone.pldb"),
        "{frame}"
    );
    assert!(frame.contains("No Ledger open."), "{frame}");
    assert!(!path.exists(), "a failed open created the file");
}

#[test]
fn open_over_an_open_ledger_that_fails_keeps_the_open_ledger() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("household.pldb");
    let notes = directory.path().join("notes.pldb");
    std::fs::write(&notes, "not a database").expect("a file to write");
    let mut shell = cold_shell(None);
    run_command(&mut shell, &format!("new {}", path.display()));
    add_gold(&mut shell);

    run_command(&mut shell, &format!("open {}", notes.display()));
    let frame = screen(&mut shell);

    assert!(
        frame.contains("\u{2717} Couldn't open ledger notes.pldb"),
        "{frame}"
    );
    assert!(
        hint_bar(&frame).trim_end().ends_with("household.pldb"),
        "{frame}"
    );
    assert!(frame.contains("xau"), "the Units View closed:\n{frame}");
}

#[test]
fn open_over_an_open_ledger_swaps_to_the_new_file_on_the_dashboard() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let first = directory.path().join("household.pldb");
    let second = directory.path().join("business.pldb");
    ledger_file(&second);
    let mut shell = cold_shell(None);
    run_command(&mut shell, &format!("new {}", first.display()));
    add_gold(&mut shell);

    run_command(&mut shell, &format!("open {}", second.display()));
    let frame = screen(&mut shell);

    assert!(frame.contains("Opened ledger business.pldb"), "{frame}");
    assert!(
        hint_bar(&frame).trim_end().ends_with("business.pldb"),
        "{frame}"
    );
    assert!(!frame.contains("xau"), "still on the Units View:\n{frame}");
}

#[test]
fn with_no_ledger_open_a_jump_is_refused_on_the_status_line() {
    let mut shell = cold_shell(None);

    press(&mut shell, "g u");
    let frame = screen(&mut shell);

    assert!(
        hint_bar(&frame).contains("No Ledger open: :open or :new first"),
        "{frame}"
    );
    assert!(frame.contains("No Ledger open."), "{frame}");
}

#[test]
fn with_no_ledger_open_a_command_is_refused_on_the_status_line() {
    for command in [
        "settings units",
        "unit",
        "dashboard",
        "close",
        "report list",
    ] {
        let mut shell = cold_shell(None);

        run_command(&mut shell, command);
        let frame = screen(&mut shell);

        assert!(
            hint_bar(&frame).contains("No Ledger open: :open or :new first"),
            ":{command} wasn't refused:\n{frame}"
        );
        assert!(!frame.contains("COMMAND"), "{frame}");
    }
}

#[test]
fn with_no_ledger_open_help_still_opens() {
    for keys in ["?", ": h e l p Enter"] {
        let mut shell = cold_shell(None);

        press(&mut shell, keys);
        let frame = screen(&mut shell);

        assert!(!frame.contains("No Ledger open."), "{keys}:\n{frame}");
        assert!(!frame.contains("first"), "{keys} was refused:\n{frame}");
    }
}

#[test]
fn with_no_ledger_open_quit_still_quits() {
    let mut shell = cold_shell(None);

    press(&mut shell, "q");

    assert!(shell.should_quit());
}
