//! The Accounts end-to-end smoke test: the Desktop's real start-up on a temporary SQLite file.
//! Add an Account, restart, see it on the Accounts page, which reads the Ledger's copy loaded from
//! the Accounts adapter, and in the Units page's ACCOUNTS group, which reads the adapter's
//! `UnitHoldings`. Run with the other smoke tests at a parent ticket's close and before a merge to
//! `main` (`mise run end-to-end-ui smoke`).

#![expect(
    clippy::expect_used,
    reason = "test code: a pane or group missing from the page should fail the test loudly"
)]

use super::common;
use bin_desktop::shell::router::{SettingsAccountsSnapshot, SettingsUnitsSnapshot};

use bin_desktop::{WindowSize, navigation::nav::Noun};
use common::{Harness, config_for};
use gpui::TestAppContext;

/// The Accounts page's names, in the order the page lists them.
fn account_names(ui: &mut Harness<'_>) -> Vec<String> {
    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    ui.click("settings-index-Accounts");
    ui.settings::<SettingsAccountsSnapshot>().account_names
}

/// The names in the base Unit's ACCOUNTS group on the docked Units page.
fn base_unit_holdings(ui: &mut Harness<'_>) -> Vec<String> {
    ui.press("g s");
    ui.click("settings-index-Units");
    ui.click("unit-row-0");
    ui.settings::<SettingsUnitsSnapshot>()
        .pane
        .expect("docked")
        .groups
        .into_iter()
        .find_map(|group| match group {
            lib_units::presenter::PaneGroup::Accounts(accounts) => Some(accounts),
            _ => None,
        })
        .expect("an ACCOUNTS group")
        .rows
        .into_iter()
        .map(|row| row.name)
        .collect()
}

#[gpui::test]
fn an_added_account_is_there_after_a_restart(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let config = config_for(&dir.path().join("ledger.pldb"));

    let names = {
        let storage = bin_desktop::start_on(&config, common::today()).expect("a new Ledger starts");
        let writer = storage.writer.clone();
        let mut ui = Harness::with_storage(app, storage);
        let before = account_names(&mut ui);
        ui.press("l n");
        assert_eq!(ui.dialog(), Some("AddAccount"));
        ui.press("r a i n y enter");
        assert_eq!(ui.dialog(), None, "the Add Dialog confirmed");
        let names = ui.settings::<SettingsAccountsSnapshot>().account_names;
        assert_eq!(names.len(), before.len() + 1);
        assert!(names.contains(&"rainy".to_string()));
        bin_desktop::quit(&writer);
        names
    };

    let storage = bin_desktop::start_on(&config, common::today()).expect("the Ledger starts again");
    let mut ui = Harness::with_storage(app, storage);
    ui.resize_to(WindowSize::XLarge);
    assert_eq!(account_names(&mut ui), names);
    assert!(
        base_unit_holdings(&mut ui).contains(&"rainy".to_string()),
        "the Units page reads the Account from the adapter's rows"
    );
}
