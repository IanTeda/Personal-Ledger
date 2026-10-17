//! Shell chrome driven by the mouse: primary rail rows (expanded and collapsed) and the topbar's
//! rail toggle. Only the elements clicked are tagged with `debug_selector`.

use super::common;

use bin_desktop::{
    WindowSize,
    navigation::nav::{FocusZone, Noun, RailMode},
};
use common::Harness;
use gpui::TestAppContext;

#[gpui::test]
fn clicking_a_primary_rail_row_navigates_and_focuses_the_view(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("tab");

    ui.click("primary-rail-Transactions");

    assert_eq!(ui.noun(), Noun::Transactions);
    assert_eq!(ui.read(|s| s.nav().focus()), FocusZone::View);
}

#[gpui::test]
fn the_topbar_toggle_collapses_and_expands_the_rail(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    ui.click("topbar-rail-toggle");
    assert_eq!(ui.rail(), RailMode::Collapsed);
    ui.click("topbar-rail-toggle");
    assert_eq!(ui.rail(), RailMode::Expanded);
}

#[gpui::test]
fn a_collapsed_rail_row_still_navigates_on_click(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("b");

    ui.click("primary-rail-collapsed-Budgets");

    assert_eq!(ui.noun(), Noun::Budgets);
    assert_eq!(ui.rail(), RailMode::Collapsed);
}

#[gpui::test]
fn at_medium_the_topbar_toggle_expands_the_rail_until_a_breakpoint(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Medium);
    assert_eq!(ui.rail(), RailMode::Collapsed);

    ui.click("topbar-rail-toggle");
    assert_eq!(ui.rail(), RailMode::Expanded);
    ui.click("primary-rail-Budgets");
    assert_eq!(ui.noun(), Noun::Budgets);

    ui.resize_to(WindowSize::Large);
    ui.resize_to(WindowSize::Medium);
    assert_eq!(
        ui.rail(),
        RailMode::Collapsed,
        "the override dropped at Large"
    );
    ui.click("primary-rail-collapsed-Transactions");
    assert_eq!(ui.noun(), Noun::Transactions);
}
