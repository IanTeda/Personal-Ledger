//! The Institutions end-to-end smoke test: the Desktop's real start-up on a temporary SQLite file.
//! Add an Institution, restart, see it on the Institutions page, and offered by the Add account
//! dialog's Institution select, which reads the Ledger's copy loaded from the Institutions adapter.
//! Run with the other smoke tests at a parent ticket's close and before a merge to `main`
//! (`mise run end-to-end-ui smoke`).

use super::common;

use bin_desktop::navigation::nav::Noun;
use common::{Harness, config_for};
use gpui::TestAppContext;

/// The Institutions page's names, in the order the page lists them.
fn institution_names(ui: &mut Harness<'_>) -> Vec<String> {
    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    ui.click("settings-index-Institutions");
    ui.settings_institutions()
        .institutions
        .into_iter()
        .map(|row| row.0)
        .collect()
}

/// Every Institution the Add account dialog's select steps through.
fn account_select_institutions(ui: &mut Harness<'_>) -> Vec<String> {
    ui.press("g s");
    ui.click("settings-index-Accounts");
    ui.press("l n");
    assert_eq!(ui.dialog(), Some("AddAccount"));
    ui.press("tab");
    let mut seen = Vec::new();
    for _ in 0..12 {
        seen.extend(ui.account_form().institution);
        ui.press("down");
    }
    ui.press("escape");
    seen
}

#[gpui::test]
fn an_added_institution_is_there_after_a_restart(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let config = config_for(&dir.path().join("ledger.pldb"));

    let names = {
        let storage = bin_desktop::start_on(&config, common::today()).expect("a new Ledger starts");
        let writer = storage.writer.clone();
        let mut ui = Harness::with_storage(app, storage);
        institution_names(&mut ui);
        ui.click("settings-add-institution");
        ui.press("B a n k space X tab enter");
        assert_eq!(ui.dialog(), None, "the Add Dialog confirmed");
        let names = ui
            .settings_institutions()
            .institutions
            .into_iter()
            .map(|row| row.0)
            .collect::<Vec<_>>();
        assert_eq!(names.last().map(String::as_str), Some("Bank X"));
        bin_desktop::quit(&writer);
        names
    };

    let storage = bin_desktop::start_on(&config, common::today()).expect("the Ledger starts again");
    let mut ui = Harness::with_storage(app, storage);
    assert_eq!(institution_names(&mut ui), names);
    assert!(
        account_select_institutions(&mut ui).contains(&"Bank X".to_string()),
        "the Accounts page reads the Institution from the adapter's rows"
    );
}

#[gpui::test]
fn an_edited_institution_is_there_after_a_restart(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let config = config_for(&dir.path().join("ledger.pldb"));

    let names = {
        let storage = bin_desktop::start_on(&config, common::today()).expect("a new Ledger starts");
        let writer = storage.writer.clone();
        let mut ui = Harness::with_storage(app, storage);
        assert_eq!(
            institution_names(&mut ui).first().map(String::as_str),
            Some("Personal"),
            "a new Ledger starts with the Personal Institution"
        );
        ui.click("institution-edit-2");
        for _ in 0.."American Express".len() {
            ui.press("backspace");
        }
        ui.press("A m e x");
        ui.press("enter");
        assert_eq!(ui.dialog(), None, "the Edit Dialog confirmed");
        let names = institution_names(&mut ui);
        assert_eq!(names[2], "Amex");
        bin_desktop::quit(&writer);
        names
    };

    let storage = bin_desktop::start_on(&config, common::today()).expect("the Ledger starts again");
    let mut ui = Harness::with_storage(app, storage);
    assert_eq!(institution_names(&mut ui), names);
    assert!(
        account_select_institutions(&mut ui).contains(&"Amex".to_string()),
        "the Account form's select shows the new name"
    );
}

#[gpui::test]
fn a_deleted_institution_is_gone_after_a_restart(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let config = config_for(&dir.path().join("ledger.pldb"));

    let names = {
        let storage = bin_desktop::start_on(&config, common::today()).expect("a new Ledger starts");
        let writer = storage.writer.clone();
        let mut ui = Harness::with_storage(app, storage);
        let before = institution_names(&mut ui);
        assert_eq!(
            before.last().map(String::as_str),
            Some("Superannuation Fund")
        );
        ui.click("institution-delete-6");
        ui.press("S u p e r a n n u a t i o n space F u n d");
        ui.press("enter");
        assert_eq!(ui.dialog(), None, "the Delete Dialog confirmed");
        let names = institution_names(&mut ui);
        assert_eq!(names.len(), before.len() - 1);
        bin_desktop::quit(&writer);
        names
    };

    let storage = bin_desktop::start_on(&config, common::today()).expect("the Ledger starts again");
    let mut ui = Harness::with_storage(app, storage);
    assert_eq!(institution_names(&mut ui), names);
    assert!(
        !account_select_institutions(&mut ui).contains(&"Superannuation Fund".to_string()),
        "the Accounts page no longer offers it"
    );
}
