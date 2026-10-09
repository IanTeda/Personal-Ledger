//! The command palette driven by keys alone: opening and closing, filtering, selection, tab
//! completion, `^r` history and running commands. Assertions read `PaletteSnapshot` and `NavState`
//! back from `Shell`, never pixels.

#![expect(
    clippy::expect_used,
    reason = "test code: a missing palette or result should fail the test loudly"
)]

mod common;

use bin_desktop::app::PaletteSnapshot;
use bin_desktop::navigation::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;

fn palette(ui: &mut Harness<'_>) -> Option<PaletteSnapshot> {
    ui.read(|shell| shell.palette_snapshot())
}

fn open_palette(ui: &mut Harness<'_>) -> PaletteSnapshot {
    ui.press(":");
    palette(ui).expect("`:` opens the palette")
}

#[gpui::test]
fn colon_opens_an_empty_palette_and_escape_closes_it(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    assert_eq!(palette(&mut ui), None);

    let open = open_palette(&mut ui);
    assert_eq!(open.input, "");
    assert!(open.matches.len() > 1, "every command is listed at rest");
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Command);

    ui.press("escape");
    assert_eq!(palette(&mut ui), None);
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Normal);
}

#[gpui::test]
fn typing_filters_and_ranks_prefix_matches_first(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    let all = open_palette(&mut ui).matches.len();

    ui.press("b u d");

    let filtered = palette(&mut ui).expect("still open");
    assert_eq!(filtered.input, "bud");
    assert!(filtered.matches.len() < all);
    assert!(filtered.matches.iter().all(|name| !name.is_empty()));
    assert_eq!(filtered.matches.first(), Some(&"budgets"));
    assert_eq!(filtered.selected, Some("budgets"));
}

#[gpui::test]
fn backspace_widens_the_filter_again(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    open_palette(&mut ui);
    ui.press("b u d");
    let narrow = palette(&mut ui).expect("open").matches.len();

    ui.press("backspace backspace");

    let wider = palette(&mut ui).expect("open");
    assert_eq!(wider.input, "b");
    assert!(wider.matches.len() >= narrow);
}

#[gpui::test]
fn a_query_that_matches_nothing_selects_nothing(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    open_palette(&mut ui);

    ui.press("z z z q");

    let none = palette(&mut ui).expect("open");
    assert!(none.matches.is_empty());
    assert_eq!(none.selected, None);
}

#[gpui::test]
fn enter_with_no_match_just_closes_the_palette(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    open_palette(&mut ui);
    ui.press("z z z q");

    ui.press("enter");

    assert_eq!(palette(&mut ui), None);
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Normal);
    assert_eq!(ui.noun(), Noun::Dashboard);
}

#[gpui::test]
fn up_and_down_move_the_selection_and_clamp_at_the_ends(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    open_palette(&mut ui);
    ui.press("b u d");
    let names = palette(&mut ui).expect("open").matches;
    assert!(names.len() >= 2, "budgets and its verbs");

    ui.press("down");
    assert_eq!(palette(&mut ui).expect("open").selected, Some(names[1]));
    ui.press("up");
    assert_eq!(palette(&mut ui).expect("open").selected, Some(names[0]));
    ui.press("up");
    assert_eq!(
        palette(&mut ui).expect("open").selected,
        Some(names[0]),
        "clamps at the first result"
    );

    for _ in 0..names.len() + 2 {
        ui.press("down");
    }
    assert_eq!(
        palette(&mut ui).expect("open").selected,
        names.last().copied(),
        "clamps at the last result"
    );
}

#[gpui::test]
fn tab_completes_the_input_to_the_selected_command(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    open_palette(&mut ui);
    ui.press("b u d down");
    let selected = palette(&mut ui).expect("open").selected.expect("a result");

    ui.press("tab");

    assert_eq!(palette(&mut ui).expect("open").input, selected);
}

#[gpui::test]
fn enter_runs_the_selected_navigation_command_and_closes(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    open_palette(&mut ui);

    ui.press("t r a n s a c t i o n s enter");

    assert_eq!(ui.noun(), Noun::Transactions);
    assert_eq!(palette(&mut ui), None);
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Normal);
}

#[gpui::test]
fn a_palette_command_lands_where_the_g_jump_does(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("g b");
    let via_jump = ui.noun();
    ui.press("g d");

    open_palette(&mut ui);
    ui.press("b u d g e t s enter");

    assert_eq!(ui.noun(), via_jump);
}

#[gpui::test]
fn the_palette_swallows_g_jumps_and_other_normal_keys(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    open_palette(&mut ui);

    ui.press("g l");

    assert_eq!(ui.noun(), Noun::Dashboard);
    assert_eq!(palette(&mut ui).expect("open").input, "gl");
}

#[gpui::test]
fn control_r_recalls_previously_run_commands_newest_first(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    open_palette(&mut ui);
    ui.press("b i l l s enter");
    open_palette(&mut ui);
    ui.press("b u d g e t s enter");
    assert_eq!(
        ui.read(|s| s.command_history().to_vec()),
        vec!["budgets".to_string(), "bills".to_string()]
    );

    open_palette(&mut ui);
    ui.press("ctrl-r");
    assert_eq!(palette(&mut ui).expect("open").input, "budgets");
    ui.press("ctrl-r");
    assert_eq!(palette(&mut ui).expect("open").input, "bills");
    ui.press("ctrl-r");
    assert_eq!(
        palette(&mut ui).expect("open").input,
        "bills",
        "clamps at the oldest entry"
    );
}

#[gpui::test]
fn control_r_with_no_history_does_nothing(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    open_palette(&mut ui);

    ui.press("ctrl-r");

    assert_eq!(palette(&mut ui).expect("open").input, "");
}

#[gpui::test]
fn running_the_same_command_twice_keeps_one_history_entry(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    for _ in 0..2 {
        open_palette(&mut ui);
        ui.press("b i l l s enter");
    }

    assert_eq!(
        ui.read(|s| s.command_history().to_vec()),
        vec!["bills".to_string()]
    );
}
