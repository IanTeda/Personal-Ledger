//! The Payees end-to-end smoke test: the Desktop's real start-up on a temporary SQLite file. Add a
//! Payee and rename one a Bill Plan names, restart, see both on the Payees page, and the Bill Plan
//! showing its Payee by the new name, read from the Payees adapter. Run with the other smoke tests
//! at a parent ticket's close and before a merge to `main` (`mise run end-to-end-ui smoke`).

#![expect(
    clippy::expect_used,
    reason = "test support: a Telstra row the stub doesn't seed should fail the test loudly"
)]

use super::common;
use bin_desktop::shell::router::SettingsPayeesSnapshot;

use bin_desktop::navigation::nav::Noun;
use common::{Harness, config_for};
use gpui::TestAppContext;

/// Settings on the Payees page (seven presses down the index) with focus stepped into it.
fn on_payees(ui: &mut Harness<'_>) {
    ui.press("g s");
    ui.press("j j j j j j j");
    assert_eq!(ui.settings_frame().page, "Payees");
    ui.press("l");
}

/// Moves the selection to the row named `name`, wherever the A–Z order puts it.
fn select(ui: &mut Harness<'_>, name: &str) {
    let row = ui
        .settings::<SettingsPayeesSnapshot>()
        .payee_names
        .iter()
        .position(|payee| payee == name)
        .expect("the stub seeds the Payee");
    ui.press("g g");
    for _ in 0..row {
        ui.press("j");
    }
    assert_eq!(
        ui.settings::<SettingsPayeesSnapshot>()
            .selected_payee
            .as_deref(),
        Some(name)
    );
}

/// The Payee the Telstra Internet Bill Plan's Edit Dialog shows.
fn telstra_internet_payee(ui: &mut Harness<'_>) -> String {
    ui.press("g w");
    assert_eq!(ui.noun(), Noun::Bills);
    ui.press("tab");
    let row = ui
        .bills()
        .plans
        .iter()
        .position(|plan| plan == "Telstra Internet")
        .expect("the stub seeds Telstra Internet");
    ui.press("g g");
    for _ in 0..row {
        ui.press("j");
    }
    ui.press("e");
    assert_eq!(ui.bills().plan_name.as_deref(), Some("Telstra Internet"));
    let payee = ui
        .bills()
        .plan_payee
        .expect("the Edit Dialog shows the Plan's Payee");
    ui.press("escape");
    payee
}

#[gpui::test]
fn an_added_and_a_renamed_payee_are_there_after_a_restart(app: &mut TestAppContext) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let config = config_for(&dir.path().join("ledger.pldb"));

    let names = {
        let storage = bin_desktop::start_on(&config, common::today()).expect("a new Ledger starts");
        let writer = storage.writer.clone();
        let mut ui = Harness::with_storage(app, storage);
        assert_eq!(telstra_internet_payee(&mut ui), "Telstra");
        on_payees(&mut ui);
        ui.press("n z o o enter");
        assert_eq!(ui.dialog(), None, "the Add Dialog confirmed");
        select(&mut ui, "Telstra");
        ui.press("e x enter");
        assert_eq!(ui.dialog(), None, "the Edit Dialog confirmed");
        let names = ui.settings::<SettingsPayeesSnapshot>().payee_names;
        assert!(names.iter().any(|name| name == "zoo"));
        assert!(names.iter().any(|name| name == "Telstrax"));
        bin_desktop::quit(&writer);
        names
    };

    let storage = bin_desktop::start_on(&config, common::today()).expect("the Ledger starts again");
    let mut ui = Harness::with_storage(app, storage);
    on_payees(&mut ui);
    assert_eq!(ui.settings::<SettingsPayeesSnapshot>().payee_names, names);
    assert_eq!(
        telstra_internet_payee(&mut ui),
        "Telstrax",
        "Bills reads the renamed Payee from the Payees adapter"
    );
}
