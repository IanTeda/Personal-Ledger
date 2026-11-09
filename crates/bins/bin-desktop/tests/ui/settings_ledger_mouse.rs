//! Settings › Ledger driven by the mouse: an index click opens that page, and the
//! Base unit select's options set the base Unit, which the Units page and new Accounts follow. State is read back from `Root`.

use super::common;
use bin_desktop::shell::router::SettingsLedgerSnapshot;

use bin_desktop::navigation::nav::Noun;
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
        Some("aud")
    );

    ui.click("settings-ledger-base-unit-vas");
    assert_eq!(
        ui.settings::<SettingsLedgerSnapshot>().base_unit.as_deref(),
        Some("vas")
    );
    ui.click("settings-ledger-base-unit-aud");
    assert_eq!(
        ui.settings::<SettingsLedgerSnapshot>().base_unit.as_deref(),
        Some("aud")
    );
}

#[gpui::test]
fn the_units_page_marks_the_unit_picked_on_ledger(app: &mut TestAppContext) {
    let mut ui = on_settings(app);
    assert_eq!(ui.base_unit_code().as_deref(), Some("aud"));
    ui.click("settings-index-Ledger");

    ui.click("settings-ledger-base-unit-btc");
    assert_eq!(ui.base_unit_code().as_deref(), Some("btc"));
}

#[gpui::test]
fn a_deactivated_unit_has_no_base_unit_option_to_click(app: &mut TestAppContext) {
    let mut ui = on_settings(app);
    assert!(
        ui.settings::<SettingsLedgerSnapshot>()
            .options
            .contains(&"btc".to_string())
    );

    ui.click("settings-index-Units");
    ui.right_click("unit-row-1");
    ui.click("settings-units-menu-delete-unit");
    // A Unit in use opens its Delete dialog as Deactivate.
    ui.click("delete-unit-confirm");
    ui.click("settings-index-Ledger");

    assert_eq!(
        ui.settings::<SettingsLedgerSnapshot>().options,
        vec!["aud", "vas"]
    );
    ui.click("settings-ledger-base-unit-vas");
    assert_eq!(
        ui.settings::<SettingsLedgerSnapshot>().base_unit.as_deref(),
        Some("vas")
    );
}
