//! The Settings View's own clicks: an index entry hands focus back to the index, and a button
//! with nothing behind it says so on the status line. State is read back from `Shell`.

use super::common;

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
fn clicking_an_index_entry_from_a_page_focuses_the_index(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    ui.press("l");
    assert!(ui.settings().page_focused);
    ui.click("settings-index-Display");
    let page = ui.settings();
    assert_eq!(page.page, "Display");
    assert!(!page.page_focused);
}

#[gpui::test]
fn clicking_add_price_source_opens_its_dialog(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    ui.click("settings-index-Units");
    ui.click("settings-add-price-source");
    assert_eq!(ui.dialog(), Some("AddPriceSource"));
}

#[gpui::test]
fn clicking_a_unit_in_the_base_unit_select_makes_it_the_base_unit(app: &mut TestAppContext) {
    let mut ui = on_settings(app);
    assert_eq!(ui.settings_general().base_unit.as_deref(), Some("aud"));

    ui.click("settings-general-base-unit-vas");
    assert_eq!(ui.settings_general().base_unit.as_deref(), Some("vas"));
    ui.click("settings-general-base-unit-aud");
    assert_eq!(ui.settings_general().base_unit.as_deref(), Some("aud"));
}

#[gpui::test]
fn the_units_page_marks_the_unit_picked_on_general(app: &mut TestAppContext) {
    let mut ui = on_settings(app);
    assert_eq!(ui.settings_units().base_code.as_deref(), Some("aud"));

    ui.click("settings-general-base-unit-btc");
    assert_eq!(ui.settings_units().base_code.as_deref(), Some("btc"));
}

#[gpui::test]
fn a_deactivated_unit_has_no_base_unit_option_to_click(app: &mut TestAppContext) {
    let mut ui = on_settings(app);
    assert!(ui.settings_general().options.contains(&"btc".to_string()));

    ui.click("settings-index-Units");
    ui.right_click("unit-row-1");
    ui.click("settings-units-menu-delete-unit");
    ui.click("delete-unit-confirm");
    ui.click("settings-index-General");

    assert_eq!(ui.settings_general().options, vec!["aud", "vas"]);
    ui.click("settings-general-base-unit-vas");
    assert_eq!(ui.settings_general().base_unit.as_deref(), Some("vas"));
}
