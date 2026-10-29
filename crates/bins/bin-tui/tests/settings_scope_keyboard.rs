//! Keyboard tests for Settings split by scope (#910, TUI-058, TUI-059): the index groups its pages
//! under this device and this ledger, the this device pages work with no Ledger open, and the
//! this ledger pages are refused until one is.

mod common;

use common::{cold_shell, press, run_command, screen, shell, type_text};

const REFUSED: &str = "No Ledger open: :open or :new first";

/// The bottom row of a drawn frame.
fn hint_bar(frame: &str) -> &str {
    frame.lines().last().unwrap_or_default()
}

/// The row a drawn frame shows `text` on, or `None` when it isn't drawn.
fn row_of(frame: &str, text: &str) -> Option<usize> {
    frame.lines().position(|line| line.contains(text))
}

#[test]
fn the_index_groups_this_device_then_this_ledger() {
    let mut shell = shell();

    press(&mut shell, "g s");
    let frame = screen(&mut shell);

    assert!(frame.contains("17 pages"), "{frame}");
    // Each page by its description, as its name is drawn elsewhere too.
    let rows: Vec<Option<usize>> = [
        "this device",
        "on start, toasts",
        "version, licences",
        "this ledger",
        "ledger name, owner",
        "every change",
    ]
    .iter()
    .map(|text| row_of(&frame, text))
    .collect();
    assert!(rows.iter().all(Option::is_some), "{rows:?}:\n{frame}");
    assert!(rows.is_sorted(), "out of order {rows:?}:\n{frame}");
    assert!(
        !frame.contains('\u{25cf}'),
        "a scope glyph is drawn:\n{frame}"
    );
}

#[test]
fn the_ledger_page_holds_what_general_held_and_general_holds_this_device() {
    let mut shell = shell();

    run_command(&mut shell, "settings ledger");
    let ledger = screen(&mut shell);
    run_command(&mut shell, "settings general");
    let general = screen(&mut shell);

    for needle in ["Settings \u{203a} Ledger", "ledger name", "base unit"] {
        assert!(ledger.contains(needle), "{needle} missing:\n{ledger}");
    }
    for needle in [
        "Settings \u{203a} General",
        "on start",
        "reopen last ledger",
        "Toasts",
        "locale",
        "ledger folder",
        "last ledger",
        "test.pldb",
    ] {
        assert!(general.contains(needle), "{needle} missing:\n{general}");
    }
}

#[test]
fn on_start_is_chosen_on_general_and_kept_for_the_state_file() {
    let mut shell = shell();
    run_command(&mut shell, "settings general");

    press(&mut shell, "Enter Right Enter");

    assert_eq!(
        shell.client_preferences().on_start,
        lib_core::OnStart::NoLedger
    );
    assert!(screen(&mut shell).contains("start with no ledger open"));
}

#[test]
fn with_no_ledger_open_settings_opens_on_general() {
    for keys in ["g s", ": s e t t i n g s Enter"] {
        let mut shell = cold_shell(None);

        press(&mut shell, keys);
        let frame = screen(&mut shell);

        assert!(
            frame.contains("17 pages \u{b7} 5 without a Ledger"),
            "{keys}:\n{frame}"
        );
        assert!(!hint_bar(&frame).contains(REFUSED), "{keys}:\n{frame}");
        press(&mut shell, "Enter");
        assert!(
            screen(&mut shell).contains("Settings \u{203a} General"),
            "{keys}"
        );
    }
}

#[test]
fn with_no_ledger_open_a_remembered_this_ledger_page_gives_way_to_general() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("household.pldb");
    let mut shell = cold_shell(None);
    run_command(&mut shell, &format!("new {}", path.display()));
    run_command(&mut shell, "settings units");
    run_command(&mut shell, "close");

    press(&mut shell, "g s Enter");
    let frame = screen(&mut shell);

    assert!(frame.contains("Settings \u{203a} General"), "{frame}");
}

#[test]
fn with_no_ledger_open_a_remembered_this_device_page_is_kept() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("household.pldb");
    let mut shell = cold_shell(None);
    run_command(&mut shell, &format!("new {}", path.display()));
    run_command(&mut shell, "settings logs");
    run_command(&mut shell, "close");

    press(&mut shell, "g s Enter");
    let frame = screen(&mut shell);

    assert!(frame.contains("Settings \u{203a} Logs"), "{frame}");
}

#[test]
fn with_no_ledger_open_enter_on_a_this_ledger_row_is_refused() {
    let mut shell = cold_shell(None);
    press(&mut shell, "g s G");

    press(&mut shell, "Enter");
    let frame = screen(&mut shell);

    assert!(hint_bar(&frame).contains(REFUSED), "{frame}");
    assert!(frame.contains("17 pages"), "left the index:\n{frame}");
}

#[test]
fn with_no_ledger_open_brackets_stop_at_the_last_this_device_page() {
    let mut shell = cold_shell(None);
    run_command(&mut shell, "settings about");

    press(&mut shell, "]");
    let frame = screen(&mut shell);

    assert!(hint_bar(&frame).contains(REFUSED), "{frame}");
    assert!(frame.contains("Settings \u{203a} About"), "{frame}");
}

#[test]
fn with_no_ledger_open_the_picker_refuses_a_this_ledger_page() {
    let mut shell = cold_shell(None);
    run_command(&mut shell, "settings about");

    press(&mut shell, "p j Enter");
    let frame = screen(&mut shell);

    assert!(hint_bar(&frame).contains(REFUSED), "{frame}");
    assert!(frame.contains("Go to Settings Page"), "{frame}");
}

#[test]
fn with_no_ledger_open_settings_naming_a_this_ledger_page_is_refused() {
    let mut shell = cold_shell(None);

    run_command(&mut shell, "settings units");
    let frame = screen(&mut shell);

    assert!(hint_bar(&frame).contains(REFUSED), "{frame}");
    assert!(frame.contains("No Ledger open."), "{frame}");
}

#[test]
fn with_no_ledger_open_the_this_device_pages_open_by_name() {
    for (page, heading) in [
        ("general", "General"),
        ("display", "Display"),
        ("ai-connections", "AI connections"),
        ("logs", "Logs"),
        ("about", "About"),
    ] {
        let mut shell = cold_shell(None);

        run_command(&mut shell, &format!("settings {page}"));
        let frame = screen(&mut shell);

        assert!(
            frame.contains(&format!("Settings \u{203a} {heading}")),
            "{page}:\n{frame}"
        );
    }
}

#[test]
fn with_no_ledger_open_toasts_still_toggle_on_general() {
    let mut shell = cold_shell(None);
    run_command(&mut shell, "settings general");

    press(&mut shell, "j");
    type_text(&mut shell, " ");

    assert!(!shell.client_preferences().toasts_on);
}
