//! The Documents end-to-end smoke test: the Desktop's real start-up on a temporary SQLite file.
//! File an Inbox Document and rename a Document Type, restart, and see both, read from the
//! Documents adapter. Run with the other smoke tests at a parent ticket's close and before a merge
//! to `main` (`mise run end-to-end-ui smoke`).

#![expect(
    clippy::panic,
    reason = "test support: a row that never comes into focus should fail the test loudly"
)]

use super::common;

use bin_desktop::shell::router::SettingsDocumentTypesSnapshot;
use common::{Harness, config_for};
use gpui::TestAppContext;

/// The Document Type names Settings › Documents lists, with focus stepped into the page.
fn type_names(ui: &mut Harness<'_>) -> Vec<String> {
    ui.press("g s");
    ui.press("j j j j j j j j");
    assert_eq!(ui.settings_frame().page, "Documents");
    ui.press("l");
    ui.settings::<SettingsDocumentTypesSnapshot>()
        .document_type_names
}

/// The Inbox's row count, on Documents with the Inbox showing.
fn inbox_rows(ui: &mut Harness<'_>) -> usize {
    ui.press("g f");
    ui.press("i");
    ui.documents().inbox_rows
}

/// An Inbox receipt with a strong match, so picking its first row files it.
const STRONG_RECEIPT: &str = "IMG_4471.jpg";

/// Moves the list selection down with `j` until it is on `file`.
fn focus_row(ui: &mut Harness<'_>, file: &str) {
    for _ in 0..200 {
        if ui.documents().selected_file.as_deref() == Some(file) {
            return;
        }
        ui.press("j");
    }
    panic!("never reached `{file}`");
}

#[gpui::test]
fn a_filed_document_and_a_renamed_type_are_there_after_a_restart(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let config = config_for(&dir.path().join("ledger.pldb"));

    let (rows, names) = {
        let storage = bin_desktop::start_on(&config, common::today()).expect("a new Ledger starts");
        let writer = storage.writer.clone();
        let mut ui = Harness::with_storage(app, storage);

        let before = inbox_rows(&mut ui);
        focus_row(&mut ui, STRONG_RECEIPT);
        ui.press("l");
        ui.press("enter");
        let rows = ui.documents().inbox_rows;
        assert_eq!(rows, before - 1, "picking files the Document");

        type_names(&mut ui);
        ui.press("j j");
        ui.press("e");
        ui.press("backspace backspace backspace");
        ui.press("t a x e s");
        ui.press("enter");
        assert_eq!(ui.dialog(), None, "the Edit Dialog confirmed");
        let names = ui
            .settings::<SettingsDocumentTypesSnapshot>()
            .document_type_names;
        assert_eq!(names[2], "taxes");
        ui.settle_writes();
        bin_desktop::quit(&writer);
        (rows, names)
    };

    let storage = bin_desktop::start_on(&config, common::today()).expect("the Ledger starts again");
    let mut ui = Harness::with_storage(app, storage);
    assert_eq!(inbox_rows(&mut ui), rows);
    assert_eq!(type_names(&mut ui), names);
}
