//! The Create ledger end-to-end smoke test: the Desktop's real start-up and a real SQLite file.
//! `:new` creates a Ledger with a base Unit the person chose, `:close` flushes it, and a restart
//! opens the file with that base Unit and what every new Ledger starts with (#1068, ADR-0054). Run
//! with the other smoke tests (`mise run end-to-end-ui smoke`).

use super::common;

use bin_desktop::{
    navigation::nav::Noun,
    persistence::PersistedState,
    shell::router::{SettingsDocumentTypesSnapshot, SettingsInstitutionsSnapshot},
};
use common::{Harness, config_for};
use gpui::TestAppContext;

/// Settings' `page`, then back to the Dashboard.
fn on_page<'a, T>(ui: &mut Harness<'a>, page: &str, read: impl FnOnce(&mut Harness<'a>) -> T) -> T {
    ui.press("g s");
    ui.click(&format!("settings-index-{page}"));
    assert_eq!(ui.settings_frame().page, page);
    let seen = read(ui);
    ui.press("g d");
    seen
}

#[gpui::test]
fn a_created_ledger_is_there_with_its_base_unit_and_starting_rows_after_a_restart(
    app: &mut TestAppContext,
) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let path = dir.path().join("teda.pldb");
    let config = config_for(&path);

    {
        let mut ui = Harness::starting(
            app,
            bin_desktop::Started::NoLedger,
            PersistedState::default(),
        );
        ui.browse_from(dir.path());
        ui.press(": n e w enter");
        ui.press("t e d a enter");
        assert_eq!(ui.dialog(), Some("CreateLedger"));
        ui.press("backspace backspace backspace b t c");
        ui.press("tab down tab");
        ui.press(&"backspace ".repeat("US Dollar".len()));
        ui.press("B i t c o i n");
        ui.press("enter");
        assert_eq!(ui.noun(), Noun::Dashboard);
        assert_eq!(ui.base_unit_code().as_deref(), Some("btc"));
        // Closing flushes the Ledger's writes into the file, as quitting does.
        ui.press(": c l o s e enter");
        assert!(!ui.read(|shell| shell.ledger_open()));
    }

    let storage = bin_desktop::start_on(&config, common::today())
        .storage()
        .expect("the created Ledger opens");
    let mut ui = Harness::with_storage(app, storage);
    assert_eq!(
        ui.unit_rows(),
        [(
            "btc".to_string(),
            "Bitcoin".to_string(),
            "Crypto".to_string()
        )]
    );
    assert_eq!(ui.base_unit_code().as_deref(), Some("btc"));
    let institutions = on_page(&mut ui, "Institutions", |ui| {
        ui.settings::<SettingsInstitutionsSnapshot>().institutions
    });
    assert_eq!(
        institutions,
        [("Personal".to_string(), "cash".to_string())],
        "only the Personal Institution"
    );
    let types = on_page(&mut ui, "Documents", |ui| {
        ui.settings::<SettingsDocumentTypesSnapshot>()
            .document_type_names
    });
    assert_eq!(types.len(), 9, "the default Document Types");
    ui.press("g b");
    assert_eq!(ui.budgets().rows.len(), 0, "Personal spending starts empty");
    assert!(ui.dashboard_bills().is_empty(), "no sample data");
}
