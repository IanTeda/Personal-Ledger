//! Keyboard tests for the Dashboard, the view `Shell` opens on (ADR-0035 exemplar).

mod common;

use common::{press, screen, shell};

#[test]
fn cold_start_shows_the_no_ledger_note() {
    let mut shell = shell();

    let frame = screen(&mut shell);

    assert!(frame.contains("No Personal Ledger file loaded."), "{frame}");
    assert!(frame.contains(":new") && frame.contains(":open"), "{frame}");
}

#[test]
fn colon_opens_the_command_popup_and_escape_closes_it() {
    let mut shell = shell();

    press(&mut shell, ":");
    let open = screen(&mut shell);
    assert!(open.contains("· COMMAND"), "{open}");
    assert!(open.contains("esc close"), "{open}");

    press(&mut shell, "Esc");
    let closed = screen(&mut shell);
    assert!(closed.contains("NORMAL"), "{closed}");
    assert!(!closed.contains("COMMAND"), "{closed}");
}
