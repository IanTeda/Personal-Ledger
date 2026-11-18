//! The Shell driven by the mouse: primary rail rows (expanded and collapsed) and the topbar's
//! rail toggle, Back/Forward and Undo with its History list (#1245), and how the status line fits a
//! narrow window. Only the elements clicked or measured are tagged with `debug_selector`.

use super::common;

use bin_desktop::{
    WindowSize,
    navigation::nav::{FocusZone, Noun, RailMode},
    shell::router::HistoryPick,
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

/// Settings › General with three changes made in turn, On start, Start view and the language, so
/// the History holds three entries, the language's newest.
fn three_general_changes(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s l");
    ui.press("l j l j j l");
    assert_eq!(ui.read(|s| s.history()).undo.len(), 3);
    ui
}

/// The labels of the History list's Undo rows, newest first, and its Redo rows, furthest on top.
fn history_labels(ui: &mut Harness<'_>) -> (Vec<String>, Vec<String>) {
    let history = ui.read(|s| s.history());
    let labels = |rows: Vec<lib_shell::HistoryRow>| rows.into_iter().map(|row| row.label).collect();
    (labels(history.undo), labels(history.redo))
}

#[gpui::test]
fn the_caret_lists_three_changes_newest_first_with_relative_times(app: &mut TestAppContext) {
    let mut ui = three_general_changes(app);

    ui.click("topbar-undo-menu");

    assert!(ui.is_drawn("topbar-undo-list"));
    assert!(ui.is_drawn("topbar-history-now"));
    for steps in 1..=3 {
        assert!(ui.is_drawn(&format!("topbar-history-undo-{steps}")));
    }
    assert!(!ui.is_drawn("topbar-history-undo-4"));
    assert!(!ui.is_drawn("topbar-history-redo-1"));
    assert!(!ui.is_drawn("topbar-history-empty"));
    let history = ui.read(|s| s.history());
    let rows: Vec<(&str, &str)> = history
        .undo
        .iter()
        .map(|row| (row.label.as_str(), row.when.as_str()))
        .collect();
    assert_eq!(
        rows,
        [
            ("Language and region: English (Australia)", "just now"),
            ("Start view: Last view", "just now"),
            ("On start: Start with no Ledger open", "just now"),
        ]
    );
}

#[gpui::test]
fn undo_once_moves_the_newest_above_now(app: &mut TestAppContext) {
    let mut ui = three_general_changes(app);
    let (undo, _) = history_labels(&mut ui);

    ui.click("topbar-undo");
    ui.click("topbar-undo-menu");

    assert!(ui.is_drawn("topbar-history-redo-1"));
    assert!(!ui.is_drawn("topbar-history-undo-3"));
    let (now_undo, now_redo) = history_labels(&mut ui);
    assert_eq!(now_redo, [undo[0].clone()]);
    assert_eq!(now_undo, undo[1..]);
    assert_eq!(
        ui.read(|s| s.status_message()),
        Some(format!("Undid {}", undo[0]))
    );
}

#[gpui::test]
fn the_third_undo_row_undoes_all_three_and_the_top_redo_row_redoes_them(app: &mut TestAppContext) {
    let mut ui = three_general_changes(app);
    let (undo, _) = history_labels(&mut ui);
    let changed = ui.conf();

    ui.click("topbar-undo-menu");
    ui.click("topbar-history-undo-3");

    assert!(
        !ui.read(|s| s.undo_menu_open()),
        "a row click closes the list"
    );
    assert_eq!(ui.conf(), "[General]\n", "all three undone");
    assert_eq!(
        ui.read(|s| s.status_message()),
        Some(format!("Undid 3 changes, back to before {}", undo[2]))
    );
    assert!(!ui.read(|s| s.history()).can_undo);

    ui.click("topbar-undo-menu");
    ui.click("topbar-history-redo-3");

    assert!(!ui.read(|s| s.undo_menu_open()));
    assert_eq!(ui.conf(), changed, "all three redone");
    assert_eq!(
        ui.read(|s| s.status_message()),
        Some(format!("Redid 3 changes, through {}", undo[0]))
    );
    assert_eq!(history_labels(&mut ui), (undo, Vec::new()));
}

#[gpui::test]
fn hovering_a_row_highlights_every_row_its_click_steps_through(app: &mut TestAppContext) {
    let mut ui = three_general_changes(app);
    ui.click("topbar-undo-menu");

    ui.hover("topbar-history-undo-2");
    assert_eq!(ui.read(|s| s.undo_hover()), Some(HistoryPick::Undo(2)));

    ui.hover("topbar-history-undo-3");
    assert_eq!(ui.read(|s| s.undo_hover()), Some(HistoryPick::Undo(3)));

    ui.click("topbar-undo-menu");
    assert_eq!(ui.read(|s| s.undo_hover()), None, "closing forgets it");
}

#[gpui::test]
fn an_empty_history_draws_undo_inert_and_the_caret_shows_nothing_to_undo(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    assert!(!ui.read(|s| s.history()).can_undo);

    ui.click("topbar-undo");
    assert_eq!(ui.read(|s| s.status_message()), None, "Undo is inert");

    ui.click("topbar-undo-menu");
    assert!(ui.read(|s| s.undo_menu_open()));
    assert!(ui.is_drawn("topbar-history-empty"));
    assert!(!ui.is_drawn("topbar-history-undo-1"));
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

#[gpui::test]
fn at_medium_the_status_line_shortens_the_ledger_path(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    assert_eq!(ui.status(), None, "the Dashboard shows the ledger path");
    let x_large = ui.bounds("status-right").size.width;

    ui.resize_to(WindowSize::Medium);
    let medium = ui.bounds("status-right").size.width;
    assert!(
        medium < x_large,
        "{medium:?} is not shorter than {x_large:?}"
    );

    ui.resize_to(WindowSize::Large);
    assert_eq!(ui.bounds("status-right").size.width, x_large);
}

#[gpui::test]
fn at_the_medium_floor_the_status_line_drops_hints_from_the_end(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize(gpui::size(gpui::px(720.0), gpui::px(480.0)));
    // Settings › Inventory, focused: the longest legend, with Medium's page keys on the end.
    ui.press("g s");
    ui.press(&["j"; 9].join(" "));
    ui.press("enter");
    let legend = ui.status().expect("Inventory has a legend").hints.len();

    // gpui keeps the bounds of a strip drawn earlier, so only a count never drawn before proves
    // what this frame shows: the index page's legend was 2 long.
    assert!(
        !ui.is_drawn(&format!("status-page-hints-shown-{legend}")),
        "the whole {legend}-hint legend doesn't fit at 720px"
    );
    assert!(
        (3..legend).any(|shown| ui.is_drawn(&format!("status-page-hints-shown-{shown}"))),
        "the first hints still show"
    );
    let right = ui.bounds("status-right");
    assert!(
        right.right() <= gpui::px(720.0),
        "the context note keeps its place"
    );

    // Widened, the legend loses Medium's page keys, and all of what's left shows.
    ui.resize_to(WindowSize::XLarge);
    let legend = ui.status().expect("Inventory has a legend").hints.len();
    assert!(ui.is_drawn(&format!("status-page-hints-shown-{legend}")));
}
