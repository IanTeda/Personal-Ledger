//! The Settings View driven by keys, across its pages: the Settings index as the resting view at
//! every Window Size, opening a page and returning to the index, re-entry, Back and Forward, and
//! the Medium drill-down with its page picker. Each page's own
//! keys are in its `settings_<page>_keyboard.rs`. State is read back from `Root`.

use super::common;

use bin_desktop::{
    WindowSize,
    navigation::nav::{InputMode, Noun},
};
use common::Harness;
use gpui::TestAppContext;

fn mode(ui: &mut Harness<'_>) -> InputMode {
    ui.read(|shell| shell.nav().mode())
}

fn on_settings(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    ui
}

/// Settings with the page `j` presses down the index and focus stepped into it.
fn in_page<'a>(app: &'a mut TestAppContext, downs: usize, page: &str) -> Harness<'a> {
    let mut ui = on_settings(app);
    ui.press(&vec!["j"; downs].join(" "));
    assert_eq!(ui.settings_frame().page, page);
    ui.press("l");
    assert!(ui.settings_frame().page_focused);
    ui
}

#[gpui::test]
fn opens_on_general_with_focus_on_the_index(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    let page = ui.settings_frame();
    assert_eq!(page.page, "General");
    assert!(!page.page_focused);
    assert!(ui.is_drawn("settings-index-page"));
    assert!(!ui.is_drawn("settings-page"), "no page is open");
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn j_and_k_move_the_index_cursor_without_opening_a_page(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    ui.press("j");
    assert_eq!(ui.settings_frame().page, "Display");
    assert!(!ui.is_drawn("settings-page"));
    ui.press("j j j");
    assert_eq!(
        ui.settings_frame().page,
        "Ledger",
        "THIS DEVICE runs into THIS LEDGER"
    );
    ui.press("k");
    assert_eq!(ui.settings_frame().page, "About");
}

#[gpui::test]
fn l_steps_into_a_page_and_h_or_escape_steps_back_out(app: &mut TestAppContext) {
    let mut ui = in_page(app, 7, "Accounts");

    ui.press("h");
    assert!(!ui.settings_frame().page_focused);
    ui.press("enter");
    assert!(ui.settings_frame().page_focused);
    ui.press("escape");
    assert!(!ui.settings_frame().page_focused);
}

#[gpui::test]
fn the_page_on_show_survives_leaving_and_returning(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    ui.press("j j j j j j j");
    assert_eq!(ui.settings_frame().page, "Accounts");
    ui.press("g d");
    assert_eq!(ui.noun(), Noun::Dashboard);
    ui.press("g s");
    let page = ui.settings_frame();
    assert_eq!(page.page, "Accounts");
    assert!(!page.page_focused);
}

/// Every page in index order: THIS DEVICE, then THIS LEDGER.
const PAGES: [&str; 15] = [
    "General",
    "Display",
    "Tracing",
    "About",
    "Ledger",
    "Units",
    "Institutions",
    "Accounts",
    "Categories",
    "Tags",
    "Payees",
    "Documents",
    "Inventory",
    "SyncServer",
    "DataBackup",
];

/// The status line's hint keys, in order.
fn hint_keys(ui: &mut Harness<'_>) -> Vec<String> {
    ui.status()
        .map(|status| status.hints.into_iter().map(|(key, _)| key).collect())
        .unwrap_or_default()
}

#[gpui::test]
fn at_medium_g_s_lands_on_the_index_page(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Medium);
    ui.press("g s");

    assert!(ui.is_drawn("settings-index-page"));
    assert!(
        !ui.is_drawn("settings-index-General"),
        "no docked index at Medium"
    );
    assert!(!ui.settings_frame().page_focused);
    assert_eq!(hint_keys(&mut ui), ["j/k", "l/enter"]);
}

/// Whether the row tagged `row` sits wholly inside the Medium index page's scroll.
fn index_row_on_show(ui: &mut Harness<'_>, row: &str) -> bool {
    let frame = ui.bounds("settings-index-page");
    let row = ui.bounds(row);
    row.top() >= frame.top() && row.bottom() <= frame.bottom()
}

#[gpui::test]
fn at_medium_j_and_k_scroll_the_index_page_to_keep_the_selected_row_on_show(
    app: &mut TestAppContext,
) {
    let mut ui = Harness::new(app);
    ui.resize(gpui::size(gpui::px(960.0), gpui::px(480.0)));
    ui.press("g s");
    assert!(index_row_on_show(&mut ui, "settings-index-page-General"));
    assert!(
        !index_row_on_show(&mut ui, "settings-index-page-About"),
        "a 480px window can't show every row"
    );

    for page in &PAGES[1..] {
        ui.press("j");
        assert!(
            index_row_on_show(&mut ui, &format!("settings-index-page-{page}")),
            "{page} scrolled into view"
        );
    }
    assert!(!index_row_on_show(&mut ui, "settings-index-page-General"));

    for page in PAGES[..PAGES.len() - 1].iter().rev() {
        ui.press("k");
        assert!(
            index_row_on_show(&mut ui, &format!("settings-index-page-{page}")),
            "{page} scrolled back into view"
        );
    }
}

#[gpui::test]
fn at_medium_esc_back_to_the_index_shows_the_page_it_left(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize(gpui::size(gpui::px(960.0), gpui::px(480.0)));
    ui.press("g s enter p k");
    ui.press(&vec!["j"; PAGES.len()].join(" "));
    ui.press("enter");
    assert_eq!(ui.settings_frame().page, "DataBackup");

    ui.press("escape");

    assert!(index_row_on_show(&mut ui, "settings-index-page-DataBackup"));
}

#[gpui::test]
fn at_medium_enter_opens_a_page_in_the_index_s_place_and_esc_goes_back(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Medium);
    ui.press("g s j enter");

    let page = ui.settings_frame();
    assert_eq!(page.page, "Display");
    assert!(page.page_focused);
    assert!(ui.is_drawn("settings-crumb-index"));
    let hints = hint_keys(&mut ui);
    assert!(hints.ends_with(&["[/]".to_string(), "p".to_string(), "esc".to_string()]));

    ui.press("escape");
    assert!(ui.is_drawn("settings-index-page"));
    assert_eq!(
        ui.settings_frame().page,
        "Display",
        "the page stays selected"
    );
    assert!(!ui.settings_frame().page_focused);
}

#[gpui::test]
fn at_medium_brackets_step_through_every_page(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Medium);
    ui.press("g s enter");

    for (index, page) in PAGES.into_iter().enumerate() {
        if index > 0 {
            ui.press("]");
        }
        let settings = ui.settings_frame();
        assert_eq!(settings.page, page);
        assert!(
            settings.page_focused,
            "{page} opens even with nothing to focus"
        );
        assert!(ui.is_drawn("settings-page"), "{page} renders at Medium");
    }
    ui.press("]");
    assert_eq!(
        ui.settings_frame().page,
        "DataBackup",
        "stops at the last page"
    );
    ui.press("[ [");
    assert_eq!(ui.settings_frame().page, "Inventory");
}

#[gpui::test]
fn at_medium_p_opens_the_picker_and_j_k_enter_jump_to_the_chosen_page(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Medium);
    ui.press("g s j enter p");

    assert_eq!(mode(&mut ui), InputMode::Filter);
    assert_eq!(
        ui.settings_frame().picker_chosen.as_deref(),
        Some("Display")
    );

    ui.press("j j j");
    assert_eq!(ui.settings_frame().picker_chosen.as_deref(), Some("Ledger"));
    ui.press("k");
    assert_eq!(ui.settings_frame().picker_chosen.as_deref(), Some("About"));
    assert_eq!(
        ui.settings_frame().page,
        "Display",
        "moving leaves the page on show"
    );

    ui.press("enter");

    let settings = ui.settings_frame();
    assert_eq!(settings.page, "About");
    assert!(settings.page_focused);
    assert_eq!(settings.picker_chosen, None);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn at_medium_the_picker_moves_with_the_arrows_and_stops_at_either_end(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Medium);
    ui.press("g s enter p");
    assert_eq!(
        ui.settings_frame().picker_chosen.as_deref(),
        Some("General")
    );

    ui.press("k up");
    assert_eq!(
        ui.settings_frame().picker_chosen.as_deref(),
        Some("General")
    );
    ui.press("down");
    assert_eq!(
        ui.settings_frame().picker_chosen.as_deref(),
        Some("Display")
    );
    ui.press("up");
    assert_eq!(
        ui.settings_frame().picker_chosen.as_deref(),
        Some("General")
    );

    ui.press(&vec!["j"; PAGES.len()].join(" "));
    assert_eq!(
        ui.settings_frame().picker_chosen.as_deref(),
        Some("DataBackup")
    );
}

#[gpui::test]
fn at_medium_the_open_picker_swallows_every_other_key(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Medium);
    ui.press("g s enter p");

    ui.press("g t ] h");

    assert_eq!(ui.noun(), Noun::Settings);
    let settings = ui.settings_frame();
    assert_eq!(settings.page, "General");
    assert_eq!(settings.picker_chosen.as_deref(), Some("General"));
    assert_eq!(mode(&mut ui), InputMode::Filter);
}

#[gpui::test]
fn at_medium_escape_closes_the_picker_and_leaves_the_page(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Medium);
    ui.press("g s j enter p");

    ui.press("escape");

    let settings = ui.settings_frame();
    assert_eq!(settings.picker_chosen, None);
    assert_eq!(settings.page, "Display");
    assert!(settings.page_focused);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn leaving_medium_closes_the_picker_and_docks_the_index(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Medium);
    ui.press("g s j enter p");

    ui.resize_to(WindowSize::Large);

    assert_eq!(ui.settings_frame().picker_chosen, None);
    assert_eq!(mode(&mut ui), InputMode::Normal);
    assert!(ui.is_drawn("settings-index-Display"));
    assert!(
        ui.settings_frame().page_focused && ui.is_drawn("settings-page"),
        "resizing keeps the page showing"
    );
}

#[gpui::test]
fn resizing_keeps_the_index_showing(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("g s j");

    for window_size in [WindowSize::Medium, WindowSize::XLarge, WindowSize::Large] {
        ui.resize_to(window_size);
        assert!(ui.is_drawn("settings-index-page"), "{window_size:?}");
        assert_eq!(ui.settings_frame().page, "Display");
    }
}

#[gpui::test]
fn at_large_and_x_large_g_s_lands_on_the_index_beside_the_docked_index(app: &mut TestAppContext) {
    for window_size in [WindowSize::Large, WindowSize::XLarge] {
        let mut ui = Harness::new(app);
        ui.resize_to(window_size);
        ui.press("g s");

        assert!(ui.is_drawn("settings-index-page"), "{window_size:?}");
        assert!(ui.is_drawn("settings-index-General"), "the index docks too");
        assert!(!ui.is_drawn("settings-page"), "no page is open");
        assert!(!ui.settings_frame().page_focused);
        assert_eq!(hint_keys(&mut ui), ["j/k", "l/enter"]);
    }
}

#[gpui::test]
fn at_large_and_x_large_every_page_opens_beside_the_docked_index(app: &mut TestAppContext) {
    for window_size in [WindowSize::Large, WindowSize::XLarge] {
        let mut ui = Harness::new(app);
        ui.resize_to(window_size);

        for (index, page) in PAGES.into_iter().enumerate() {
            // Each time from the index: `g s` lands on it even from an open page, its cursor on
            // the page just left.
            ui.press("g s");
            assert!(!ui.settings_frame().page_focused);
            if index > 0 {
                ui.press("j");
            }
            ui.press("enter");
            assert_eq!(ui.settings_frame().page, page);
            assert!(ui.settings_frame().page_focused, "{page} opens");
            assert!(ui.is_drawn(&format!("settings-index-{page}")));
            // Units' tables fill the View instead (`settings_units_keyboard`).
            assert!(
                page == "Units" || ui.bounds("settings-page").size.width <= gpui::px(840.0),
                "{page} caps its content at {window_size:?}"
            );
        }
        // `[`, `]` and `p` are Medium's; here they leave the page alone.
        ui.press("] p");
        assert_eq!(ui.settings_frame().page, "DataBackup");
        assert_eq!(ui.settings_frame().picker_chosen, None);
    }
}

#[gpui::test]
fn esc_h_and_left_return_a_page_to_the_index_at_every_window_size(app: &mut TestAppContext) {
    for window_size in [WindowSize::Medium, WindowSize::Large, WindowSize::XLarge] {
        for leave in ["escape", "h", "left"] {
            let mut ui = Harness::new(app);
            ui.resize_to(window_size);
            // About has no key of its own for `h` or `left`.
            ui.press(": s e t t i n g s space a b o u t enter");
            assert_eq!(ui.settings_frame().page, "About");
            assert!(ui.settings_frame().page_focused);

            ui.press(leave);

            let settings = ui.settings_frame();
            assert!(!settings.page_focused, "{leave} at {window_size:?}");
            assert!(
                ui.is_drawn("settings-index-page"),
                "{leave} at {window_size:?}"
            );
            assert_eq!(settings.page, "About", "the cursor stays on the page left");
        }
    }
}

/// Large or X-Large Settings on Tags, then Payees clicked in the docked index, which leaves the
/// keyboard there.
fn docked_on_payees(app: &mut TestAppContext, window_size: WindowSize) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.resize_to(window_size);
    ui.press(": s e t t i n g s space t a g s enter");
    ui.click("settings-index-Payees");
    let settings = ui.settings_frame();
    assert_eq!(settings.page, "Payees");
    assert!(settings.dock_focused, "{window_size:?}");
    ui
}

#[gpui::test]
fn at_large_and_x_large_j_and_k_on_the_docked_index_show_each_page(app: &mut TestAppContext) {
    for window_size in [WindowSize::Large, WindowSize::XLarge] {
        let mut ui = docked_on_payees(app, window_size);
        assert_eq!(hint_keys(&mut ui), ["j/k", "l/enter"], "the docked index's");

        ui.press("j");
        let settings = ui.settings_frame();
        assert_eq!(settings.page, "Documents", "{window_size:?}");
        assert!(
            settings.dock_focused,
            "the keyboard stays on the docked index"
        );
        assert!(
            ui.is_drawn("settings-page"),
            "the page under the cursor shows"
        );
        ui.press("k k");
        assert_eq!(ui.settings_frame().page, "Tags");
        assert!(ui.settings_frame().dock_focused);
    }
}

#[gpui::test]
fn at_large_and_x_large_enter_l_or_right_move_the_keyboard_from_the_docked_index_into_the_page(
    app: &mut TestAppContext,
) {
    for window_size in [WindowSize::Large, WindowSize::XLarge] {
        for into in ["enter", "l", "right"] {
            let mut ui = docked_on_payees(app, window_size);

            ui.press(into);

            let settings = ui.settings_frame();
            assert_eq!(settings.page, "Payees");
            assert!(settings.page_focused, "{into} at {window_size:?}");
            assert!(!settings.dock_focused);
        }
    }
}

#[gpui::test]
fn at_large_and_x_large_esc_on_the_docked_index_returns_to_the_index(app: &mut TestAppContext) {
    for window_size in [WindowSize::Large, WindowSize::XLarge] {
        let mut ui = docked_on_payees(app, window_size);

        ui.press("escape");

        let settings = ui.settings_frame();
        assert!(!settings.dock_focused, "{window_size:?}");
        assert!(!settings.page_focused);
        assert!(ui.is_drawn("settings-index-page"));
        assert_eq!(settings.page, "Payees", "the cursor stays on the page left");
    }
}

#[gpui::test]
fn every_way_into_settings_lands_on_the_index_even_from_a_page(app: &mut TestAppContext) {
    for enter in ["g s", ": s e t t i n g s enter", "rail"] {
        let mut ui = Harness::new(app);
        ui.press("g s j enter");
        assert!(ui.settings_frame().page_focused);

        if enter == "rail" {
            ui.click("primary-rail-Settings");
        } else {
            ui.press(enter);
        }

        let settings = ui.settings_frame();
        assert_eq!(ui.noun(), Noun::Settings);
        assert!(!settings.page_focused, "{enter}");
        assert!(ui.is_drawn("settings-index-page"), "{enter}");
        assert_eq!(settings.page, "Display", "{enter} keeps the cursor");
    }
}

#[gpui::test]
fn back_and_forward_show_the_page_itself(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("g s j enter");
    ui.press("g d");
    assert_eq!(ui.noun(), Noun::Dashboard);

    ui.click("topbar-back");
    let settings = ui.settings_frame();
    assert_eq!(ui.noun(), Noun::Settings);
    assert_eq!(settings.page, "Display");
    assert!(settings.page_focused, "Back shows the page, not the index");
    assert!(ui.is_drawn("settings-page"));

    ui.press("escape j enter");
    assert_eq!(ui.settings_frame().page, "Tracing");
    ui.click("topbar-back");
    let settings = ui.settings_frame();
    assert_eq!(
        settings.page, "Display",
        "history records pages, not the index"
    );
    assert!(settings.page_focused);

    ui.click("topbar-forward");
    let settings = ui.settings_frame();
    assert_eq!(settings.page, "Tracing");
    assert!(settings.page_focused);
}

#[gpui::test]
fn closing_the_ledger_on_a_this_ledger_page_shows_the_index_on_general(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press(": s e t t i n g s space u n i t s enter");
    assert_eq!(ui.settings_frame().page, "Units");
    assert!(ui.settings_frame().page_focused);

    ui.press(": c l o s e enter");
    ui.press("g s");

    let settings = ui.settings_frame();
    assert_eq!(settings.page, "General");
    assert!(!settings.page_focused);
    assert!(ui.is_drawn("settings-index-page"));
}

/// The top of the element tagged `selector`.
fn top(ui: &mut Harness<'_>, selector: &str) -> gpui::Pixels {
    ui.bounds(selector).top()
}

#[gpui::test]
fn the_docked_index_groups_this_device_above_this_ledger(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    let device = top(&mut ui, "settings-index-group-ThisDevice");
    let ledger = top(&mut ui, "settings-index-group-ThisLedger");
    assert!(device < top(&mut ui, "settings-index-General"));
    assert!(top(&mut ui, "settings-index-About") < ledger);
    assert!(ledger < top(&mut ui, "settings-index-Ledger"));
}

#[gpui::test]
fn at_medium_the_index_page_heads_each_group_with_its_scope_line(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Medium);
    ui.press("g s");

    let device = top(&mut ui, "settings-index-page-group-ThisDevice");
    let ledger = top(&mut ui, "settings-index-page-group-ThisLedger");
    assert!(device < top(&mut ui, "settings-index-page-General"));
    assert!(top(&mut ui, "settings-index-page-About") < ledger);
    assert!(ledger < top(&mut ui, "settings-index-page-Ledger"));
}

#[gpui::test]
fn at_medium_the_picker_lists_this_device_then_this_ledger(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Medium);
    ui.press("g s enter p");

    let tops: Vec<_> = PAGES
        .iter()
        .map(|page| top(&mut ui, &format!("settings-picker-{page}")))
        .collect();
    assert!(tops.windows(2).all(|pair| pair[0] < pair[1]), "index order");
    // The THIS LEDGER heading opens a gap wider than a row between About and Ledger.
    let row = tops[1] - tops[0];
    assert!(tops[4] - tops[3] > row);
}

#[gpui::test]
fn with_no_ledger_open_a_bracket_onto_a_this_ledger_page_is_refused(app: &mut TestAppContext) {
    let mut ui = Harness::without_ledger(app);
    ui.resize_to(WindowSize::Medium);
    ui.press("g s enter");
    ui.press("] ] ]");
    assert_eq!(ui.settings_frame().page, "About");

    ui.press("]");
    let settings = ui.settings_frame();
    assert_eq!(settings.page, "About", "the page on show stays");
    assert!(settings.page_focused);
    assert_eq!(settings.status_message.as_deref(), Some(common::NO_LEDGER));
}

#[gpui::test]
fn with_no_ledger_open_settings_on_a_this_ledger_page_is_refused(app: &mut TestAppContext) {
    let mut ui = Harness::without_ledger(app);
    ui.press("g s");

    ui.press(": s e t t i n g s space u n i t s enter");
    let settings = ui.settings_frame();
    assert_eq!(settings.page, "General");
    assert!(!settings.page_focused);
    assert_eq!(settings.status_message.as_deref(), Some(common::NO_LEDGER));
}

#[gpui::test]
fn with_no_ledger_open_enter_in_the_picker_on_a_this_ledger_page_is_refused(
    app: &mut TestAppContext,
) {
    let mut ui = Harness::without_ledger(app);
    ui.resize_to(WindowSize::Medium);
    ui.press("g s enter p");
    ui.press("j j j j");
    assert_eq!(ui.settings_frame().picker_chosen.as_deref(), Some("Ledger"));

    ui.press("enter");
    let settings = ui.settings_frame();
    assert_eq!(settings.page, "General");
    assert_eq!(
        settings.picker_chosen.as_deref(),
        Some("Ledger"),
        "the picker stays open"
    );
    assert_eq!(settings.status_message.as_deref(), Some(common::NO_LEDGER));
}

#[gpui::test]
fn with_no_ledger_open_a_refused_settings_command_leaves_back_where_it_was(
    app: &mut TestAppContext,
) {
    let mut ui = Harness::without_ledger(app);
    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);

    ui.press(": s e t t i n g s space u n i t s enter");
    assert_eq!(
        ui.settings_frame().status_message.as_deref(),
        Some(common::NO_LEDGER)
    );

    ui.click("topbar-back");
    assert_eq!(
        ui.noun(),
        Noun::Dashboard,
        "a refused page open records nothing, so Back goes to where the person was before"
    );
}

#[gpui::test]
fn an_accepted_settings_page_open_steps_back_and_forward_through_the_page_it_left(
    app: &mut TestAppContext,
) {
    let mut ui = Harness::new(app);
    ui.press("g s");
    ui.press(": s e t t i n g s space u n i t s enter");
    assert_eq!(ui.settings_frame().page, "Units");

    ui.click("topbar-back");
    assert_eq!(ui.noun(), Noun::Settings);
    assert_eq!(ui.settings_frame().page, "General");

    ui.click("topbar-back");
    assert_eq!(ui.noun(), Noun::Dashboard);

    ui.click("topbar-forward");
    assert_eq!(ui.noun(), Noun::Settings);
    assert_eq!(ui.settings_frame().page, "General");

    ui.click("topbar-forward");
    assert_eq!(ui.noun(), Noun::Settings);
    assert_eq!(ui.settings_frame().page, "Units");
}

#[gpui::test]
fn at_x_large_the_index_docks_a_detail_pane_that_follows_j_and_k(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::XLarge);
    ui.press("g s");

    assert!(ui.is_drawn("settings-index-pane"));
    assert!(ui.is_drawn("settings-index-pane-General"));
    assert_eq!(ui.settings_frame().pane.as_deref(), Some("General"));
    let pane = ui.bounds("settings-index-pane");
    assert_eq!(
        pane.size.width,
        gpui::px(400.0),
        "the list pages' dock width"
    );
    assert!(
        ui.bounds("settings-index-page").right() <= pane.left(),
        "the index spans the View up to the pane"
    );

    ui.press("j");
    assert_eq!(ui.settings_frame().pane.as_deref(), Some("Display"));
    assert!(ui.is_drawn("settings-index-pane-Display"));
    ui.press("j j j");
    assert_eq!(ui.settings_frame().pane.as_deref(), Some("Ledger"));
    assert!(ui.settings_frame().pane_open_enabled);
    ui.press("k");
    assert_eq!(ui.settings_frame().pane.as_deref(), Some("About"));
    assert!(
        !ui.settings_frame().page_focused,
        "the pane never takes focus"
    );
    assert_eq!(
        hint_keys(&mut ui),
        ["j/k", "l/enter"],
        "the status line is the index's"
    );
}

#[gpui::test]
fn at_x_large_enter_opens_the_page_the_pane_shows_and_no_pane_docks_beside_a_page(
    app: &mut TestAppContext,
) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::XLarge);
    ui.press("g s j j j j j");
    assert_eq!(ui.settings_frame().pane.as_deref(), Some("Units"));

    ui.press("enter");

    let settings = ui.settings_frame();
    assert_eq!(settings.page, "Units");
    assert!(settings.page_focused);
    assert_eq!(
        settings.pane, None,
        "a page shows, so the index's pane goes"
    );

    ui.press("escape");
    assert_eq!(ui.settings_frame().pane.as_deref(), Some("Units"));
}

#[gpui::test]
fn at_medium_and_large_the_index_docks_no_pane(app: &mut TestAppContext) {
    for window_size in [WindowSize::Medium, WindowSize::Large] {
        let mut ui = Harness::new(app);
        ui.resize_to(window_size);
        ui.press("g s j");

        assert!(ui.is_drawn("settings-index-page"), "{window_size:?}");
        assert!(!ui.is_drawn("settings-index-pane"), "{window_size:?}");
        assert_eq!(ui.settings_frame().pane, None);
    }
}

#[gpui::test]
fn resizing_to_x_large_docks_the_pane_and_back_to_large_takes_it_away(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Large);
    ui.press("g s j");
    assert_eq!(ui.settings_frame().pane, None);

    ui.resize_to(WindowSize::XLarge);
    assert_eq!(ui.settings_frame().pane.as_deref(), Some("Display"));
    assert!(ui.is_drawn("settings-index-pane-Display"));

    ui.resize_to(WindowSize::Large);
    assert_eq!(ui.settings_frame().pane, None);
}

/// The pane would disable **Open** on a This Ledger page with no Ledger open, but the index's
/// cursor never lands on one then (#977), so the pane only ever offers a page that opens.
#[gpui::test]
fn with_no_ledger_open_the_pane_follows_the_cursor_through_the_this_device_pages_only(
    app: &mut TestAppContext,
) {
    let mut ui = Harness::without_ledger(app);
    ui.resize_to(WindowSize::XLarge);
    ui.press("g s");
    assert_eq!(ui.settings_frame().pane.as_deref(), Some("General"));
    assert!(ui.settings_frame().pane_open_enabled);

    ui.press("j j j j j");

    let settings = ui.settings_frame();
    assert_eq!(settings.pane.as_deref(), Some("About"), "the cursor stops");
    assert!(settings.pane_open_enabled);
    ui.press("enter");
    assert_eq!(ui.settings_frame().page, "About");
    assert!(ui.settings_frame().page_focused);
}
