//! The Unit prices view on Settings › Units driven by clicks, on in-memory Units ports. Wiring
//! only: the Prices menu entry and pane button open it, and a click on a week's row selects it.

use super::common;
use bin_desktop::shell::router::UnitPricesSnapshot;

use bin_desktop::WindowSize;
use common::Harness;
use gpui::TestAppContext;
use lib_core::UnitId;

fn on_units_at(app: &mut TestAppContext, size: WindowSize) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.resize_to(size);
    ui.press("g s");
    ui.click("settings-index-Units");
    assert_eq!(ui.settings_frame().page, "Units");
    ui
}

#[gpui::test]
fn the_prices_menu_entry_opens_the_rows_unit(app: &mut TestAppContext) {
    let mut ui = on_units_at(app, WindowSize::Large);
    ui.right_click("unit-row-1");
    ui.click("settings-units-menu-prices");

    let prices = ui
        .settings::<Option<UnitPricesSnapshot>>()
        .expect("the menu entry opens it");
    assert_eq!(prices.unit, UnitId::SEED_BTC);
}

#[gpui::test]
fn the_pane_button_opens_the_selected_units_prices(app: &mut TestAppContext) {
    let mut ui = on_units_at(app, WindowSize::XLarge);
    ui.click("unit-row-1");
    ui.click("units-pane-prices");

    assert_eq!(
        ui.settings::<Option<UnitPricesSnapshot>>()
            .expect("open")
            .unit,
        UnitId::SEED_BTC
    );
    assert!(ui.is_drawn("unit-prices-chart"));
}

#[gpui::test]
fn a_click_on_a_weeks_row_selects_it(app: &mut TestAppContext) {
    let mut ui = on_units_at(app, WindowSize::Large);
    ui.right_click("unit-row-1");
    ui.click("settings-units-menu-prices");

    ui.click("unit-prices-row-2");
    assert!(
        ui.settings::<Option<UnitPricesSnapshot>>()
            .expect("open")
            .rows[2]
            .selected
    );
}
