//! Shell chrome driven by the mouse: primary rail rows (expanded and collapsed) and the topbar's
//! rail toggle, Back/Forward and Undo with its recent-actions list. Only the elements clicked are tagged with `debug_selector`.

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

#[gpui::test]
fn topbar_back_and_forward_retrace_the_nouns_visited(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    assert!(!ui.read(|s| s.nav().can_go_back()));

    ui.click("primary-rail-Transactions");
    ui.click("primary-rail-Budgets");

    ui.click("topbar-back");
    assert_eq!(ui.noun(), Noun::Transactions);
    ui.click("topbar-back");
    assert_eq!(ui.noun(), Noun::Dashboard);
    ui.click("topbar-back");
    assert_eq!(ui.noun(), Noun::Dashboard, "Back does nothing at the start");

    ui.click("topbar-forward");
    assert_eq!(ui.noun(), Noun::Transactions);

    ui.click("primary-rail-Bills");
    ui.click("topbar-forward");
    assert_eq!(ui.noun(), Noun::Bills, "a new noun drops what was ahead");
}

#[gpui::test]
fn the_undo_caret_lists_the_last_six_actions(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    assert!(!ui.is_drawn("topbar-undo-list"));

    ui.click("topbar-undo-menu");

    assert!(ui.read(|s| s.undo_menu_open()));
    assert!(ui.is_drawn("topbar-undo-list"));
    for index in 0..6 {
        assert!(ui.is_drawn(&format!("topbar-undo-action-{index}")));
    }
    assert!(!ui.is_drawn("topbar-undo-action-6"));
}

#[gpui::test]
fn picking_a_recent_action_closes_the_list_and_reports_undo_unbuilt(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.click("topbar-undo-menu");

    ui.click("topbar-undo-action-2");

    assert!(!ui.read(|s| s.undo_menu_open()));
    assert!(ui.read(|s| s.status_message().is_some()));
}

#[gpui::test]
fn the_undo_button_reports_undo_unbuilt(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    ui.click("topbar-undo");

    assert!(ui.read(|s| s.status_message().is_some()));
}

#[gpui::test]
fn clicking_outside_the_undo_list_or_its_caret_again_closes_it(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    ui.click("topbar-undo-menu");
    ui.click("topbar-undo-menu");
    assert!(
        !ui.read(|s| s.undo_menu_open()),
        "the caret toggles it shut"
    );

    ui.click("topbar-undo-menu");
    ui.click("primary-rail-Budgets");
    assert!(!ui.read(|s| s.undo_menu_open()));
    assert_eq!(
        ui.noun(),
        Noun::Dashboard,
        "the click outside only closes the list"
    );
}
