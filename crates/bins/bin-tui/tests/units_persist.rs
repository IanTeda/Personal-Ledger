//! The TUI's one test on a real Ledger file: start-up opens it as `main` does, a Unit added in the
//! Units View is saved, and a second start on the same file shows it. The Desktop opens the same
//! file through the same `lib-database` adapter, which is what lets the two Clients agree.

mod common;

use common::{press, screen, shell_on, type_text};

#[test]
fn a_unit_added_in_the_tui_is_there_after_a_restart() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    let path = directory.path().join("ledger.pldb");

    let storage = lib_database::open_storage(&path).expect("a new Ledger opens and is seeded");
    let (mut shell, _report) = shell_on(Box::new(storage.units), std::rc::Rc::new(storage.pending));
    press(&mut shell, "g u n");
    type_text(&mut shell, "xau");
    press(&mut shell, "Tab Tab");
    type_text(&mut shell, "Gold");
    press(&mut shell, "Enter");
    drop(shell);
    storage.writer.flush().expect("the write lands");
    drop(storage.writer);

    let storage = lib_database::open_storage(&path).expect("the Ledger opens again");
    let (mut shell, _report) = shell_on(Box::new(storage.units), std::rc::Rc::new(storage.pending));
    press(&mut shell, "g u");
    let frame = screen(&mut shell);

    assert!(frame.contains("xau"), "{frame}");
    assert!(
        frame.contains("aud"),
        "the seed isn't written twice:\n{frame}"
    );
}
