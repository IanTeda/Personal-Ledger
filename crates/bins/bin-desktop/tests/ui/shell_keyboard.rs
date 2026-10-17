//! Shell chrome driven by keys alone: `g` jumps, the primary rail's browse-and-commit, focus
//! zone cycling, the rail toggle and the modes the key router enters and leaves. Assertions read
//! `NavState` back from `Shell`, never pixels.

use super::common;

use bin_desktop::{
    WindowSize,
    navigation::nav::{FocusZone, InputMode, Noun, RailMode},
};
use common::Harness;
use gpui::TestAppContext;

fn focus(ui: &mut Harness<'_>) -> FocusZone {
    ui.read(|shell| shell.nav().focus())
}

fn mode(ui: &mut Harness<'_>) -> InputMode {
    ui.read(|shell| shell.nav().mode())
}

fn rail(ui: &mut Harness<'_>) -> RailMode {
    ui.rail()
}

#[gpui::test]
fn starts_on_dashboard_in_normal_mode_with_focus_in_the_view(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    assert_eq!(ui.noun(), Noun::Dashboard);
    assert_eq!(focus(&mut ui), FocusZone::View);
    assert_eq!(mode(&mut ui), InputMode::Normal);
    assert_eq!(rail(&mut ui), RailMode::Expanded);
}

#[gpui::test]
fn every_g_jump_lands_on_its_noun(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    for (key, noun) in bin_desktop::navigation::key_router::JUMPS {
        ui.press(&format!("g {key}"));
        assert_eq!(ui.noun(), noun, "g {key}");
    }
}

#[gpui::test]
fn the_header_is_the_wordmark_alone_on_every_view(app: &mut TestAppContext) {
    for window_size in [WindowSize::Medium, WindowSize::Large] {
        let mut ui = Harness::new(app);
        ui.resize_to(window_size);
        let wordmark = ui.bounds("topbar-brand").size;
        for (key, _) in bin_desktop::navigation::key_router::JUMPS {
            ui.press(&format!("g {key}"));
            assert_eq!(
                ui.bounds("topbar-brand").size,
                wordmark,
                "g {key} at {window_size:?}: no View suffix"
            );
        }
    }
}

#[gpui::test]
fn g_followed_by_an_unbound_key_changes_nothing(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("g l");

    ui.press("g z");

    assert_eq!(ui.noun(), Noun::Transactions);
    // The chord was consumed: a bare `l` is not half of a new one.
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn escape_clears_a_pending_g(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    ui.press("g escape l");

    assert_eq!(ui.noun(), Noun::Dashboard, "`l` after Esc is not a jump");
}

#[gpui::test]
fn tab_cycles_focus_between_the_view_and_the_primary_rail(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("g l");

    ui.press("tab");
    assert_eq!(focus(&mut ui), FocusZone::PrimaryRail);
    ui.press("tab");
    assert_eq!(focus(&mut ui), FocusZone::View);
    ui.press("shift-tab");
    assert_eq!(focus(&mut ui), FocusZone::PrimaryRail);
}

#[gpui::test]
fn tab_reaches_the_context_rail_only_once_a_ledger_is_open(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("g r");

    ui.press("tab");
    assert_eq!(focus(&mut ui), FocusZone::PrimaryRail);
    ui.press("tab");
    assert_eq!(focus(&mut ui), FocusZone::View, "no context rail yet");

    ui.shell
        .update(ui.cx, |shell, _| shell.open_ledger_for_test());
    ui.press("tab");
    assert_eq!(focus(&mut ui), FocusZone::PrimaryRail);
    ui.press("tab");
    assert_eq!(focus(&mut ui), FocusZone::ContextRail);
    ui.press("tab");
    assert_eq!(focus(&mut ui), FocusZone::View);
    ui.press("shift-tab");
    assert_eq!(focus(&mut ui), FocusZone::ContextRail);
}

#[gpui::test]
fn the_primary_rail_browses_without_committing_until_enter(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("tab");

    ui.press("j j");
    assert_eq!(ui.read(|s| s.nav().primary_highlight()), Noun::Documents);
    assert_eq!(
        ui.noun(),
        Noun::Dashboard,
        "browsing alone does not navigate"
    );
    assert_eq!(focus(&mut ui), FocusZone::PrimaryRail);

    ui.press("enter");
    assert_eq!(ui.noun(), Noun::Documents);
    assert_eq!(focus(&mut ui), FocusZone::View);
}

#[gpui::test]
fn the_primary_rail_jumps_to_its_first_and_last_rows(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("tab");

    ui.press("shift-g");
    assert_eq!(ui.read(|s| s.nav().primary_highlight()), Noun::Settings);
    ui.press("g g");
    assert_eq!(ui.read(|s| s.nav().primary_highlight()), Noun::Dashboard);
    ui.press("k");
    assert_eq!(
        ui.read(|s| s.nav().primary_highlight()),
        Noun::Dashboard,
        "clamps at the first row"
    );
}

#[gpui::test]
fn b_toggles_the_primary_rail(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    ui.press("b");
    assert_eq!(rail(&mut ui), RailMode::Collapsed);
    ui.press("b");
    assert_eq!(rail(&mut ui), RailMode::Expanded);
}

#[gpui::test]
fn the_rail_is_icons_at_medium_and_expanded_at_large(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    ui.resize_to(WindowSize::Medium);
    assert_eq!(rail(&mut ui), RailMode::Collapsed);
    assert!(ui.is_drawn("primary-rail-collapsed-Budgets"));

    ui.resize_to(WindowSize::Large);
    assert_eq!(rail(&mut ui), RailMode::Expanded);
    assert!(ui.is_drawn("primary-rail-Budgets"));
}

#[gpui::test]
fn b_holds_only_until_the_window_crosses_a_breakpoint(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Medium);

    ui.press("b");
    assert_eq!(
        rail(&mut ui),
        RailMode::Expanded,
        "b expands the Medium rail"
    );

    ui.resize_to(WindowSize::Large);
    assert_eq!(rail(&mut ui), RailMode::Expanded, "Large's own default");
    ui.press("b");
    assert_eq!(rail(&mut ui), RailMode::Collapsed);

    ui.resize_to(WindowSize::Medium);
    assert_eq!(rail(&mut ui), RailMode::Collapsed, "Medium's own default");
    ui.resize_to(WindowSize::Large);
    assert_eq!(
        rail(&mut ui),
        RailMode::Expanded,
        "the override made at Large dropped when the window left Large"
    );
}

#[gpui::test]
fn start_with_the_rail_hidden_applies_at_large_and_x_large(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    let shell = ui.shell.clone();
    shell.update(ui.cx, |shell, cx| {
        shell.set_start_sidebar_minimised(true, cx);
        cx.notify();
    });

    for window_size in [WindowSize::XLarge, WindowSize::Large, WindowSize::Medium] {
        ui.resize_to(window_size);
        assert_eq!(rail(&mut ui), RailMode::Collapsed, "at {window_size:?}");
    }

    ui.resize_to(WindowSize::Large);
    ui.press("b");
    assert_eq!(rail(&mut ui), RailMode::Expanded, "b still overrides it");
}

#[gpui::test]
fn the_palette_search_and_help_modes_enter_and_leave_with_escape(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("tab");

    for (keys, expected) in [
        (":", InputMode::Command),
        ("/", InputMode::Search),
        ("?", InputMode::Dialog),
        ("a", InputMode::Insert),
    ] {
        ui.press(keys);
        assert_eq!(mode(&mut ui), expected, "{keys}");
        ui.press("escape");
        assert_eq!(mode(&mut ui), InputMode::Normal, "escape after {keys}");
        assert_eq!(
            focus(&mut ui),
            FocusZone::PrimaryRail,
            "focus restored after {keys}"
        );
    }
}

#[gpui::test]
fn help_swallows_keys_and_a_second_question_mark_closes_it(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    ui.press("?");
    ui.press("g l b");
    assert_eq!(
        ui.noun(),
        Noun::Dashboard,
        "nothing behind the overlay reacts"
    );
    assert_eq!(mode(&mut ui), InputMode::Dialog);

    ui.press("?");
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn esc_closes_the_undo_list_and_does_nothing_else(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("g l");
    ui.click("topbar-undo-menu");

    ui.press("escape");

    assert!(!ui.read(|s| s.undo_menu_open()));
    assert_eq!(ui.noun(), Noun::Transactions);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}
