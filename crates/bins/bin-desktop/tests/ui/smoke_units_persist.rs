//! The end-to-end smoke test: the Desktop's real start-up on a temporary SQLite file. Start, add
//! a Unit, restart, see it. The only UI test off the in-memory ports, so it proves start-up,
//! persistence and restart together; run it at a parent ticket's close and before a merge to
//! `main` (`mise run end-to-end-ui smoke`).

use super::common;

use bin_desktop::navigation::nav::Noun;
use common::Harness;
use gpui::TestAppContext;

/// Opens the Units page and adds `code`, a Currency, through its Add Dialog.
fn add_unit(ui: &mut Harness<'_>, code: &str, name: &str) {
    ui.press("g s j j l");
    assert_eq!(ui.noun(), Noun::Settings);
    assert_eq!(ui.settings().page, "Units");
    ui.press("n");
    type_word(ui, code);
    ui.press("tab tab");
    type_word(ui, name);
    ui.press("enter");
    assert_eq!(ui.dialog(), None, "the Add Unit Dialog confirmed");
}

fn type_word(ui: &mut Harness<'_>, word: &str) {
    let keys: Vec<String> = word.chars().map(|ch| ch.to_string()).collect();
    ui.press(&keys.join(" "));
}

#[gpui::test]
fn a_unit_added_on_a_new_ledger_is_there_after_a_restart(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("ledger.pldb");

    {
        let storage = bin_desktop::open_storage(&path).expect("a new Ledger opens");
        let writer = storage.writer.clone();
        let mut ui = Harness::with_units(app, Box::new(storage.units));
        assert_eq!(
            ui.unit_codes(),
            ["aud", "btc", "vas"],
            "a new Ledger starts with the stub's Units"
        );
        assert_eq!(ui.base_unit_code().as_deref(), Some("aud"));

        add_unit(&mut ui, "nzd", "New Zealand dollar");
        assert!(ui.unit_codes().contains(&"nzd".to_string()));
        // What quitting the app does, so the write isn't lost with the writer thread.
        writer.flush().expect("the writer is running");
    }

    let storage = bin_desktop::open_storage(&path).expect("the Ledger opens again");
    let mut ui = Harness::with_units(app, Box::new(storage.units));
    assert_eq!(ui.unit_codes(), ["aud", "btc", "vas", "nzd"]);
}
