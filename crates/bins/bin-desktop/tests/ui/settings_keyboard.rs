//! The Settings View driven by keys, across its pages: the index rail's paged navigation, focus
//! stepping into and out of a page, and the Medium drill-down with its page picker. Each page's own
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
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn j_and_k_swap_the_page_live(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    ui.press("j");
    assert_eq!(ui.settings_frame().page, "Display");
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
}

#[gpui::test]
fn at_large_and_x_large_every_page_renders_beside_the_docked_index(app: &mut TestAppContext) {
    for window_size in [WindowSize::Large, WindowSize::XLarge] {
        let mut ui = Harness::new(app);
        ui.resize_to(window_size);
        ui.press("g s");

        for (index, page) in PAGES.into_iter().enumerate() {
            if index > 0 {
                ui.press("j");
            }
            assert_eq!(ui.settings_frame().page, page);
            assert!(ui.is_drawn(&format!("settings-index-{page}")));
            // Units' tables fill the View instead (`settings_units_keyboard`).
            assert!(
                page == "Units" || ui.bounds("settings-page").size.width <= gpui::px(840.0),
                "{page} caps its content at {window_size:?}"
            );
        }
        // `[`, `]` and `p` are Medium's; here they leave the page alone.
        ui.press("l ] p");
        assert_eq!(ui.settings_frame().page, "DataBackup");
        assert_eq!(ui.settings_frame().picker_chosen, None);
    }
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
