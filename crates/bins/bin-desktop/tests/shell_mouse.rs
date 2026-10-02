//! Shell chrome driven by the mouse: primary rail rows (expanded and collapsed) and the topbar's
//! rail toggle. Only the elements clicked are tagged with `debug_selector`.

mod common;

use bin_desktop::nav::{FocusZone, Noun, RailMode};
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
    assert_eq!(ui.read(|s| s.nav().primary_rail()), RailMode::Collapsed);
    ui.click("topbar-rail-toggle");
    assert_eq!(ui.read(|s| s.nav().primary_rail()), RailMode::Expanded);
}

#[gpui::test]
fn a_collapsed_rail_row_still_navigates_on_click(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("b");

    ui.click("primary-rail-collapsed-Budgets");

    assert_eq!(ui.noun(), Noun::Budgets);
    assert_eq!(ui.read(|s| s.nav().primary_rail()), RailMode::Collapsed);
}
