//! The command palette driven by keys alone: opening and closing, filtering, selection, tab
//! completion, `^r` history and running commands. Assertions read `PaletteSnapshot` and `NavState`
//! back from `Root`, never pixels.

#![expect(
    clippy::expect_used,
    reason = "test code: a missing palette or result should fail the test loudly"
)]

use super::common;
use bin_desktop::shell::router::UnitPricesSnapshot;

use bin_desktop::navigation::nav::{InputMode, Noun};
use bin_desktop::shell::router::PaletteSnapshot;
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

/// Types `text` into whatever has the keys, a space as `space`.
fn type_text(ui: &mut Harness<'_>, text: &str) {
    for ch in text.chars() {
        ui.press(&if ch == ' ' {
            "space".to_string()
        } else {
            ch.to_string()
        });
    }
}

/// Runs `command` from a freshly opened palette.
fn run(ui: &mut Harness<'_>, command: &str) {
    open_palette(ui);
    type_text(ui, command);
    assert_eq!(
        palette(ui).expect("open").selected,
        Some(command),
        "{command} is the top match"
    );
    ui.press("enter");
}

/// The key column `name`'s row draws while the palette is open at rest.
fn key_of(ui: &mut Harness<'_>, name: &str) -> String {
    let open = open_palette(ui);
    let row = open
        .matches
        .iter()
        .position(|matched| *matched == name)
        .expect("the command is listed");
    ui.press("escape");
    open.keys[row].clone()
}

#[gpui::test]
fn a_rebound_key_shows_in_the_palette(app: &mut TestAppContext) {
    let mut ui = Harness::with_keybindings(app, common::overriding("jump_dashboard", "g h"));

    assert_eq!(key_of(&mut ui, "dashboard"), "g h");
    assert_eq!(key_of(&mut ui, "units set-base"), "B");
}

#[gpui::test]
fn an_unbound_command_shows_a_dash(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    assert_eq!(key_of(&mut ui, "toasts"), "\u{2014}");
}

#[gpui::test]
fn the_global_keymaps_commands_are_listed(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    let open = open_palette(&mut ui);

    for name in [
        "help",
        "search",
        "toggle-rail",
        "focus-next",
        "focus-previous",
        "dismiss all",
        "toasts",
    ] {
        assert!(open.matches.contains(&name), "{name}");
    }
    for hidden in ["back", "insert", "open_command_popup"] {
        assert!(!open.matches.contains(&hidden), "{hidden}");
    }
}

#[gpui::test]
fn help_runs_from_the_palette(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    run(&mut ui, "help");

    assert!(ui.read(|s| s.help_open()));
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Dialog);
}

#[gpui::test]
fn search_runs_from_the_palette(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("g l");

    run(&mut ui, "search");

    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Search);
}

#[gpui::test]
fn toggle_rail_runs_from_the_palette(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    let before = ui.rail();

    run(&mut ui, "toggle-rail");

    assert_ne!(ui.rail(), before);
}

#[gpui::test]
fn focus_next_and_focus_previous_run_from_the_palette(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    let start = ui.read(|s| s.nav().focus());

    run(&mut ui, "focus-next");
    let next = ui.read(|s| s.nav().focus());
    assert_ne!(next, start);

    run(&mut ui, "focus-previous");
    assert_eq!(ui.read(|s| s.nav().focus()), start);
}

#[gpui::test]
fn units_set_base_runs_on_the_units_page_from_another_view_in_one_input(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    assert_eq!(ui.noun(), Noun::Dashboard);
    open_palette(&mut ui);
    type_text(&mut ui, "units set-base");
    let commits = ui.commits();

    ui.press("enter");

    assert_eq!(ui.noun(), Noun::Settings);
    assert_eq!(ui.dialog(), Some("SetBaseUnit"));
    assert_eq!(ui.read(|s| s.nav().mode()), InputMode::Dialog);
    assert_eq!(
        ui.commits() - commits,
        1,
        "the event is sent once, as `B` sends it"
    );
}

#[gpui::test]
fn payees_delete_runs_on_the_payees_page(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    run(&mut ui, "payees delete");

    assert_eq!(ui.dialog(), Some("DeletePayee"));
}

#[gpui::test]
fn bills_pay_runs_on_the_row_the_bills_page_last_had_selected(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("g w");
    let row = ui
        .bills()
        .rows
        .iter()
        .position(|row| row.plan == "Telstra Internet")
        .expect("Telstra Internet is due this month");
    for _ in 0..row {
        ui.press("j");
    }
    ui.press("g d");

    run(&mut ui, "bills pay");

    assert_eq!(ui.noun(), Noun::Bills);
    assert_eq!(ui.bills().dialog.as_deref(), Some("Pay"));
}

#[gpui::test]
fn tags_merge_runs_the_tags_merge_binding(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    run(&mut ui, "tags merge");

    assert_eq!(ui.dialog(), Some("MergeTags"));
}

#[gpui::test]
fn messages_still_opens_the_toast_history(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);

    run(&mut ui, "messages");

    assert!(ui.toasts().history_open);
}

#[gpui::test]
fn a_bills_command_of_the_other_tab_shows_that_tab_first(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("g w");
    assert_eq!(ui.bills().tab, "Schedule");

    run(&mut ui, "bills edit");

    assert_eq!(ui.bills().tab, "Planner");
    assert_eq!(ui.bills().dialog.as_deref(), Some("Edit"));
}

#[gpui::test]
fn a_units_command_closes_an_open_prices_view_first(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("g s j j l j enter");
    assert!(
        ui.settings::<Option<UnitPricesSnapshot>>().is_some(),
        "enter opens the prices view"
    );

    run(&mut ui, "units set-base");

    assert!(ui.settings::<Option<UnitPricesSnapshot>>().is_none());
    assert_eq!(ui.dialog(), Some("SetBaseUnit"));
}
