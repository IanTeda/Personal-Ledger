//! The Shell driven by keys alone: `g` jumps, the primary rail's browse-and-commit, focus
//! zone cycling, the rail toggle, the modes the global keys enter and leave, and the order a key
//! is offered in (`bin_desktop`'s `input::route_key`), each step winning a key a later one also
//! binds, the status message answering one key, and the hints naming rebound keys. Assertions read
//! `NavState` back from `Root`, never pixels.

use std::time::Duration;

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

/// Every `g` jump's default chord and the Noun it lands on.
fn jumps() -> impl Iterator<Item = (&'static str, Noun)> {
    lib_shell::keymap::GLOBAL_BINDINGS
        .iter()
        .filter_map(|binding| match binding.event {
            lib_shell::ShellCommand::Jump(noun) => Some((binding.chord, noun)),
            _ => None,
        })
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

    for (chord, noun) in jumps() {
        ui.press(chord);
        assert_eq!(ui.noun(), noun, "{chord}");
    }
}

#[gpui::test]
fn the_header_is_the_wordmark_alone_on_every_view(app: &mut TestAppContext) {
    for window_size in [WindowSize::Medium, WindowSize::Large] {
        let mut ui = Harness::new(app);
        ui.resize_to(window_size);
        let wordmark = ui.bounds("topbar-brand").size;
        for (chord, _) in jumps() {
            ui.press(chord);
            assert_eq!(
                ui.bounds("topbar-brand").size,
                wordmark,
                "{chord} at {window_size:?}: no View suffix"
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
fn a_g_jump_survives_a_slow_second_key_on_the_wall_clock(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    ui.press("g");
    // Real time only: the executor's clock stands still, so the pending `g` is still live.
    std::thread::sleep(bin_desktop::shell::router::PENDING_G_TIMEOUT + Duration::from_millis(100));
    ui.press("l");

    assert_eq!(ui.noun(), Noun::Transactions);
}

#[gpui::test]
fn a_pending_g_lapses_once_the_executor_clock_passes_its_timeout(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    ui.press("g");
    ui.advance_clock(bin_desktop::shell::router::PENDING_G_TIMEOUT + Duration::from_millis(1));
    ui.press("l");

    assert_eq!(
        ui.noun(),
        Noun::Dashboard,
        "`l` after a lapsed `g` is not a jump"
    );
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

// The key route: each step wins a key a later step also binds.

/// Takes `ui` to the Settings Units page, a module's View, with keyboard focus on it.
fn to_units(ui: &mut Harness<'_>) {
    ui.press("g s j j l");
    assert_eq!(ui.settings().page, "Units");
    assert!(ui.settings().page_focused);
}

#[gpui::test]
fn an_open_dialog_takes_a_key_the_shell_keeps(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    to_units(&mut ui);
    ui.press("n");
    let before = rail(&mut ui);

    ui.press("b");

    assert_eq!(
        ui.dialog_value("code").as_deref(),
        Some("b"),
        "typed, not the rail toggle"
    );
    assert_eq!(rail(&mut ui), before);
}

#[gpui::test]
fn the_palette_takes_a_key_the_shell_keeps(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    let before = rail(&mut ui);

    ui.press(": b");

    let input = ui.read(|shell| shell.palette_snapshot().map(|palette| palette.input));
    assert_eq!(input.as_deref(), Some("b"));
    assert_eq!(rail(&mut ui), before);
}

#[gpui::test]
fn a_key_the_shell_keeps_wins_over_a_module_binding_on_it(app: &mut TestAppContext) {
    let mut ui = Harness::with_keybindings(app, common::overriding("new", "b"));
    to_units(&mut ui);
    let before = rail(&mut ui);

    ui.press("b");

    assert_eq!(ui.dialog(), None, "the rail toggle, not New unit");
    assert_ne!(rail(&mut ui), before);
}

#[gpui::test]
fn a_module_binding_wins_over_its_hosts_own_key(app: &mut TestAppContext) {
    let mut ui = Harness::with_keybindings(app, common::overriding("new", "h"));
    to_units(&mut ui);

    ui.press("h");

    assert_eq!(ui.dialog(), Some("AddUnit"));
    ui.press("escape");
    assert!(
        ui.settings().page_focused,
        "Settings' own `h` didn't step back to the index"
    );
}

#[gpui::test]
fn a_views_own_key_wins_over_the_shells_fallback(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("g f");

    ui.press("a");
    assert_ne!(
        mode(&mut ui),
        InputMode::Insert,
        "Documents' `a`, not Insert"
    );

    ui.press("escape g b tab");
    assert_eq!(
        focus(&mut ui),
        FocusZone::View,
        "Budgets' `tab` switches its tab"
    );
}

#[gpui::test]
fn the_shell_falls_back_on_a_key_no_view_takes(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    ui.press("a");
    assert_eq!(mode(&mut ui), InputMode::Insert);

    ui.press("escape g l tab");
    assert_eq!(focus(&mut ui), FocusZone::PrimaryRail);
    ui.press("end");
    assert_eq!(
        ui.read(|shell| shell.nav().primary_highlight()),
        Noun::Settings,
        "`end` moves the rail as it moves a list"
    );
}

#[gpui::test]
fn an_override_moves_a_global_key_and_frees_its_default(app: &mut TestAppContext) {
    let mut config = common::overriding("toggle_rail", "ctrl+b");
    config
        .bindings
        .insert("jump_settings".to_string(), "g z".to_string());
    let mut ui = Harness::with_keybindings(app, config);
    let before = rail(&mut ui);

    ui.press("b");
    assert_eq!(rail(&mut ui), before, "`b` is no longer bound");
    ui.press("ctrl-b");
    assert_ne!(rail(&mut ui), before);

    ui.press("g z");
    assert_eq!(ui.noun(), Noun::Settings);
}

#[gpui::test]
fn a_status_message_answers_its_key_and_clears_on_the_next(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    let status = |ui: &mut Harness<'_>| ui.read(|shell| shell.status_message());

    ui.press("g x");
    let message = status(&mut ui).expect("`g x` says it is no jump");
    assert!(message.contains("g x"), "{message}");

    ui.press("j");
    assert_eq!(status(&mut ui), None, "the next key clears it");
}

#[gpui::test]
fn the_shell_hints_name_a_rebound_key(app: &mut TestAppContext) {
    let mut ui = Harness::with_keybindings(app, common::overriding("toggle_rail", "ctrl+b"));

    let keys: Vec<String> = ui
        .read(|shell| shell.shell_hints_snapshot())
        .into_iter()
        .map(|(key, _)| key)
        .collect();

    assert_eq!(keys, [":", "/", "?", "ctrl+b"]);
}
