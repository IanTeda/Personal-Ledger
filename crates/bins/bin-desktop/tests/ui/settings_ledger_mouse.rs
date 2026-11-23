//! Settings › Ledger driven by the mouse: an index click opens that page, a click on the Ledger
//! name or Owner edits it and a click away saves it, and the Financial year starts and Base unit
//! selects open on a click and save the option clicked; the base Unit is followed by the Units
//! page. State is read back from `Root`.

use super::common;
use bin_desktop::shell::router::SettingsLedgerSnapshot;

use bin_desktop::navigation::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;

fn on_settings(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    ui
}

#[gpui::test]
fn clicking_an_index_entry_from_a_page_opens_that_page(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    ui.press("l");
    assert!(ui.settings_frame().page_focused);
    ui.click("settings-index-Display");
    let page = ui.settings_frame();
    assert_eq!(page.page, "Display");
    assert!(
        page.dock_focused,
        "the keyboard stays on the docked index (#1110)"
    );
    assert!(ui.is_drawn("display-fields"));
}

#[gpui::test]
fn clicking_a_unit_in_the_base_unit_select_makes_it_the_base_unit(app: &mut TestAppContext) {
    let mut ui = on_settings(app);
    ui.click("settings-index-Ledger");
    assert_eq!(
        ui.settings::<SettingsLedgerSnapshot>().base_unit.as_deref(),
        Some("AUD")
    );

    ui.click("settings-ledger-base-unit");
    ui.click("settings-ledger-base-unit-option-2");
    assert_eq!(
        ui.settings::<SettingsLedgerSnapshot>().base_unit.as_deref(),
        Some("VAS")
    );
    ui.click("settings-ledger-base-unit");
    ui.click("settings-ledger-base-unit-option-0");
    assert_eq!(
        ui.settings::<SettingsLedgerSnapshot>().base_unit.as_deref(),
        Some("AUD")
    );
}

#[gpui::test]
fn the_units_page_marks_the_unit_picked_on_ledger(app: &mut TestAppContext) {
    let mut ui = on_settings(app);
    assert_eq!(ui.base_unit_code().as_deref(), Some("AUD"));
    ui.click("settings-index-Ledger");

    ui.click("settings-ledger-base-unit");
    ui.click("settings-ledger-base-unit-option-1");
    assert_eq!(ui.base_unit_code().as_deref(), Some("BTC"));
}

#[gpui::test]
fn a_deactivated_unit_has_no_base_unit_option_to_click(app: &mut TestAppContext) {
    let mut ui = on_settings(app);
    assert!(
        ui.settings::<SettingsLedgerSnapshot>()
            .options
            .contains(&"BTC".to_string())
    );

    ui.click("settings-index-Units");
    ui.right_click("unit-row-1");
    ui.click("settings-units-menu-delete-unit");
    // A Unit in use opens its Delete dialog as Deactivate.
    ui.click("delete-unit-confirm");
    ui.click("settings-index-Ledger");

    assert_eq!(
        ui.settings::<SettingsLedgerSnapshot>().options,
        vec!["AUD", "VAS"]
    );
    ui.click("settings-ledger-base-unit");
    ui.click("settings-ledger-base-unit-option-1");
    assert_eq!(
        ui.settings::<SettingsLedgerSnapshot>().base_unit.as_deref(),
        Some("VAS")
    );
}

fn ledger(ui: &mut Harness<'_>) -> SettingsLedgerSnapshot {
    ui.settings::<SettingsLedgerSnapshot>()
}

fn on_ledger(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = on_settings(app);
    ui.click("settings-index-Ledger");
    ui
}

#[gpui::test]
fn clicking_the_name_edits_it_and_a_click_away_saves_it(app: &mut TestAppContext) {
    let mut ui = on_ledger(app);

    ui.click("settings-ledger-name");
    assert!(ledger(&mut ui).editing);
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Insert);
    ui.press("x");
    ui.click("settings-ledger");

    let page = ledger(&mut ui);
    assert!(!page.editing);
    assert_eq!(page.name, "Sample Ledgerx");
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Normal);
}

#[gpui::test]
fn clicking_another_field_saves_the_draft_and_opens_that_field(app: &mut TestAppContext) {
    let mut ui = on_ledger(app);

    ui.click("settings-ledger-name");
    ui.press("x");
    ui.click("settings-ledger-owner");

    let page = ledger(&mut ui);
    assert_eq!(page.name, "Sample Ledgerx");
    assert!(page.editing, "the Owner is being edited now");
    assert_eq!(page.focused, Some("owner"));
}

#[gpui::test]
fn clicking_an_index_entry_saves_an_open_draft(app: &mut TestAppContext) {
    let mut ui = on_ledger(app);

    ui.click("settings-ledger-owner");
    ui.press("x");
    ui.click("settings-index-Units");
    ui.click("settings-index-Ledger");

    assert_eq!(ledger(&mut ui).owner, "Sam Samplex");
}

#[gpui::test]
fn a_rail_click_away_saves_an_open_draft(app: &mut TestAppContext) {
    let mut ui = on_ledger(app);

    ui.click("settings-ledger-name");
    ui.press("x");
    ui.click("primary-rail-Dashboard");
    assert_eq!(ui.noun(), Noun::Dashboard);
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Normal);

    ui.press("u");
    assert_eq!(
        ui.read(|shell| shell.status_message()).as_deref(),
        Some("Undid Ledger name changed to Sample Ledgerx"),
        "the save is a History entry of its own"
    );
}

#[gpui::test]
fn the_financial_year_select_opens_on_a_click_and_saves_the_month_clicked(
    app: &mut TestAppContext,
) {
    let mut ui = on_ledger(app);
    assert_eq!(ledger(&mut ui).financial_year.as_deref(), Some("July"));

    ui.click("settings-ledger-financial-year");
    assert!(ledger(&mut ui).list_open);
    ui.click("settings-ledger-financial-year-option-4");

    let page = ledger(&mut ui);
    assert!(!page.list_open);
    assert_eq!(page.financial_year.as_deref(), Some("May"));
}

#[gpui::test]
fn a_click_away_from_an_open_list_saves_its_highlight(app: &mut TestAppContext) {
    let mut ui = on_ledger(app);

    ui.click("settings-ledger-financial-year");
    ui.press("k");
    ui.click("settings-ledger");

    let page = ledger(&mut ui);
    assert!(!page.list_open);
    assert_eq!(page.financial_year.as_deref(), Some("June"));
}
