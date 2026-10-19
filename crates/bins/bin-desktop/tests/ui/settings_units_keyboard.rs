//! The Settings Units page driven by keys: reaching the page down the index, one selection walked
//! across both tables by `j`/`k` and jumped between them by `tab`, the page keys acting on the
//! selected row's table, and the Add, Edit and Delete unit and price source dialogs' keys once
//! open: typing, `tab` between fields, `left`/`right` on a segmented control, `enter` to save or
//! confirm, and `escape` to cancel. State is read back from `Shell`.

use super::common;

use bin_desktop::{
    WindowSize,
    navigation::nav::{InputMode, Noun},
};
use common::Harness;
use gpui::TestAppContext;

/// Most cases run at Large; the X-Large cases below drive the detail pane.
fn on_units(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Large);
    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    ui.press("j j");
    assert_eq!(ui.settings().page, "Units");
    ui
}

fn mode(ui: &mut Harness<'_>) -> InputMode {
    ui.read(|shell| shell.nav().mode())
}

/// Right-clicks the row and picks an entry of its context menu: the rows carry no buttons.
fn row_action(ui: &mut Harness<'_>, row: &str, action: &str) {
    ui.right_click(row);
    ui.click(&format!("settings-units-menu-{action}"));
}

#[gpui::test]
fn j_j_from_the_index_swaps_to_units_and_l_steps_into_it(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    assert!(!ui.settings().page_focused);

    ui.press("l");
    assert!(ui.settings().page_focused);
    assert_eq!(ui.dialog(), None);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn add_unit_types_into_code_then_tab_moves_to_name_and_enter_saves(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let before = ui.settings_units().unit_rows.len();

    ui.click("settings-add-unit");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Code"));
    ui.press("x y z");
    ui.press("tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Type"));
    ui.press("tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Name"));
    ui.press("t e s t");
    assert_eq!(ui.dialog_value("code").as_deref(), Some("xyz"));
    assert_eq!(ui.dialog_value("name").as_deref(), Some("test"));
    ui.press("enter");
    let units_page = ui.settings_units();
    assert_eq!(ui.dialog(), None);
    assert_eq!(units_page.unit_rows.len(), before + 1);
    assert_eq!(
        units_page.unit_rows.last(),
        Some(&(
            "xyz".to_string(),
            "test".to_string(),
            "currency".to_string()
        ))
    );
}

#[gpui::test]
fn tab_cycles_back_to_code_and_backspace_trims_the_focused_field(app: &mut TestAppContext) {
    let mut ui = on_units(app);

    ui.click("settings-add-unit");
    ui.press("a b");
    ui.press("tab tab tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Source"));
    ui.press("tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Code"));
    ui.press("backspace");
    assert_eq!(ui.dialog_value("code").as_deref(), Some("a"));
}

#[gpui::test]
fn add_unit_enter_does_nothing_until_code_and_name_are_filled(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let before = ui.settings_units().unit_rows.len();

    ui.click("settings-add-unit");
    ui.press("a enter");
    assert_eq!(ui.dialog(), Some("AddUnit"));
    assert_eq!(ui.settings_units().unit_rows.len(), before);
}

#[gpui::test]
fn escape_discards_an_add_unit_draft(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let before = ui.settings_units().unit_rows.len();

    ui.click("settings-add-unit");
    ui.press("a tab tab b");
    ui.press("escape");
    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.settings_units().unit_rows.len(), before);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn edit_unit_saves_a_changed_name_with_enter(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let (code, name, kind) = ui.settings_units().unit_rows[0].clone();

    row_action(&mut ui, "unit-row-0", "edit-unit");
    assert_eq!(ui.dialog(), Some("EditUnit"));
    ui.press("tab tab");
    ui.press("x");
    ui.press("enter");
    assert_eq!(ui.dialog(), None);
    assert_eq!(
        ui.settings_units().unit_rows[0],
        (code, format!("{name}x"), kind)
    );
}

#[gpui::test]
fn escape_discards_an_edit_unit_change(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let before = ui.settings_units().unit_rows.clone();

    row_action(&mut ui, "unit-row-0", "edit-unit");
    ui.press("tab tab");
    ui.press("x");
    ui.press("escape");
    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.settings_units().unit_rows, before);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn delete_unit_enter_refuses_until_the_code_is_typed_back(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let before = ui.settings_units().unit_codes.clone();
    let code = before[0].clone();

    row_action(&mut ui, "unit-row-0", "delete-unit");
    assert_eq!(ui.dialog(), Some("DeleteUnit"));
    ui.press("enter");
    assert_eq!(ui.dialog(), Some("DeleteUnit"));
    assert_eq!(ui.settings_units().unit_codes, before);

    // A partial code is still refused, so the row stays until the whole code matches.
    ui.press(
        &code
            .chars()
            .next()
            .expect("seeded units have codes")
            .to_string(),
    );
    ui.press("enter");
    assert_eq!(ui.dialog(), Some("DeleteUnit"));
    assert_eq!(ui.settings_units().unit_codes, before);

    for ch in code.chars().skip(1) {
        ui.press(&ch.to_string());
    }
    assert_eq!(ui.dialog_value("confirm").as_deref(), Some(code.as_str()));
    ui.press("enter");
    let after = ui.settings_units().unit_codes;
    assert_eq!(ui.dialog(), None);
    assert_eq!(after.len(), before.len() - 1);
    assert!(!after.contains(&code));
}

#[gpui::test]
fn escape_closes_the_delete_unit_dialog_without_a_write(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let before = ui.settings_units().unit_codes.clone();
    let code = before[0].clone();

    row_action(&mut ui, "unit-row-0", "delete-unit");
    for ch in code.chars() {
        ui.press(&ch.to_string());
    }
    ui.press("escape");
    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.settings_units().unit_codes, before);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

fn on_units_xlarge(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::XLarge);
    ui.press("g s");
    ui.press("j j");
    assert_eq!(ui.settings().page, "Units");
    ui.press("l");
    assert!(ui.settings().page_focused);
    ui
}

#[gpui::test]
fn at_x_large_j_and_k_walk_the_units_and_the_pane_follows(app: &mut TestAppContext) {
    let mut ui = on_units_xlarge(app);
    assert!(ui.is_drawn("units-detail-pane"));
    assert_eq!(ui.settings_units().selected_code.as_deref(), Some("aud"));

    ui.press("j");
    assert_eq!(ui.settings_units().selected_code.as_deref(), Some("btc"));
    ui.press("j");
    assert_eq!(ui.settings_units().selected_code.as_deref(), Some("vas"));
    ui.press("k");
    assert_eq!(ui.settings_units().selected_code.as_deref(), Some("btc"));
}

#[gpui::test]
fn at_x_large_e_opens_the_edit_dialog_for_the_selected_unit(app: &mut TestAppContext) {
    let mut ui = on_units_xlarge(app);
    ui.press("j");
    ui.press("e");
    assert_eq!(ui.dialog(), Some("EditUnit"));
    assert_eq!(ui.dialog_value("code").as_deref(), Some("btc"));
    ui.press("escape");
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn at_x_large_n_opens_the_add_dialog(app: &mut TestAppContext) {
    let mut ui = on_units_xlarge(app);
    ui.press("n");
    assert_eq!(ui.dialog(), Some("AddUnit"));
}

#[gpui::test]
fn at_x_large_d_deletes_the_selected_unit_into_activity_without_a_toast(app: &mut TestAppContext) {
    let mut ui = on_units_xlarge(app);
    ui.press("j");
    ui.press("d");
    assert_eq!(ui.dialog(), Some("DeleteUnit"));
    ui.press("b t c");
    ui.press("enter");

    let page = ui.settings_units();
    assert_eq!(ui.dialog(), None);
    assert!(!page.unit_codes.contains(&"btc".to_string()));
    assert_eq!(page.activity, vec!["Deleted unit btc".to_string()]);
    assert!(ui.toasts().visible.is_empty());
}

#[gpui::test]
fn at_x_large_t_says_the_source_test_is_not_built_for_a_unit_with_a_source(
    app: &mut TestAppContext,
) {
    let mut ui = on_units_xlarge(app);
    ui.press("t");
    assert_eq!(
        ui.read(|shell| shell.status_message().map(str::to_string)),
        None,
        "aud has no price source, so there is nothing to test"
    );
    ui.press("j");
    ui.press("t");
    assert!(
        ui.read(|shell| shell.status_message().map(str::to_string))
            .is_some()
    );
}

#[gpui::test]
fn at_large_the_page_keys_act_on_the_selected_unit_without_the_pane(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.press("l");
    assert!(!ui.is_drawn("units-detail-pane"));
    ui.press("n");
    assert_eq!(ui.dialog(), Some("AddUnit"));
    ui.press("escape");
    ui.press("j e");
    assert_eq!(ui.dialog(), Some("EditUnit"));
    assert_eq!(ui.dialog_value("code").as_deref(), Some("btc"));
    ui.press("escape");
    ui.press("d");
    assert_eq!(ui.dialog(), Some("DeleteUnit"));
}

#[gpui::test]
fn escape_closes_an_open_row_menu_before_leaving_the_page(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.press("l");
    ui.right_click("unit-row-0");
    assert!(ui.settings_units().menu.is_some());

    ui.press("escape");
    assert_eq!(ui.settings_units().menu, None);
    assert!(ui.settings().page_focused);
}

/// Both tables fill the page from Medium's floor to X-Large, past the 840px other pages cap at,
/// and never draw narrower than the page at the floor.
#[gpui::test]
fn both_tables_fill_the_page_from_the_medium_floor_to_x_large(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Medium);
    ui.press("g s j j enter");
    assert_eq!(ui.settings().page, "Units");
    // The page's width in a window at Medium's 720px floor, where the tables stop shrinking.
    const MIN_WIDTH: gpui::Pixels = gpui::px(610.0);
    let floor = gpui::size(gpui::px(720.0), gpui::px(800.0));
    let mut widths = Vec::new();
    for resize in [
        Some(floor),
        None,
        Some(WindowSize::Large.reference_frame()),
        Some(WindowSize::XLarge.reference_frame()),
    ] {
        if let Some(size) = resize {
            ui.resize(size);
        } else {
            ui.resize_to(WindowSize::Medium);
        }
        let page = ui.bounds("settings-page").size.width;
        for table in ["settings-units-table", "settings-price-sources-table"] {
            let width = ui.bounds(table).size.width;
            assert_eq!(width, page, "{table} fills the page");
            assert!(width >= MIN_WIDTH, "{table} keeps its minimum");
        }
        widths.push(page);
    }
    assert_eq!(widths[0], MIN_WIDTH);
    assert!(
        widths[3] > gpui::px(840.0),
        "X-Large draws past the Settings cap"
    );
}

// ---------------------------------------------------------------------------------------------
// Price Sources (16z·custom, 16z·edit, 16z·del)
// ---------------------------------------------------------------------------------------------

fn type_word(ui: &mut Harness<'_>, word: &str) {
    let keys: Vec<String> = word.chars().map(|ch| ch.to_string()).collect();
    ui.press(&keys.join(" "));
}

fn source_names(ui: &mut Harness<'_>) -> Vec<String> {
    ui.settings_units()
        .source_rows
        .into_iter()
        .map(|(name, _, _)| name)
        .collect()
}

/// Fills Add price source by keys alone: Name, then `tab` past Updates to Address, Request and
/// Price path.
fn fill_custom_source(ui: &mut Harness<'_>, name: &str) {
    ui.click("settings-add-price-source");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Name"));
    type_word(ui, name);
    ui.press("tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Updates"));
    for field in ["Address", "Request", "PricePath"] {
        ui.press("tab");
        assert_eq!(ui.dialog_value("field").as_deref(), Some(field));
        type_word(ui, "x");
    }
}

#[gpui::test]
fn add_price_source_fills_by_tab_and_enter_saves_it_in_order(app: &mut TestAppContext) {
    let mut ui = on_units(app);

    ui.click("settings-add-price-source");
    ui.press("enter");
    assert_eq!(
        ui.dialog(),
        Some("AddPriceSource"),
        "enter waits for the required fields"
    );
    ui.press("escape");

    fill_custom_source(&mut ui, "zebra");
    ui.press("shift-tab shift-tab shift-tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Updates"));
    ui.press("left");
    assert_eq!(ui.dialog_value("updates").as_deref(), Some("Hourly"));
    ui.press("enter");

    assert_eq!(ui.dialog(), None);
    assert_eq!(
        source_names(&mut ui).last().map(String::as_str),
        Some("zebra")
    );
}

#[gpui::test]
fn api_key_authentication_asks_for_a_key_it_masks_and_never_stores(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    fill_custom_source(&mut ui, "broker");

    ui.press("tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Auth"));
    ui.press("right");
    assert_eq!(ui.dialog_value("auth").as_deref(), Some("ApiKey"));
    ui.press("tab");
    type_word(&mut ui, "apikey");
    ui.press("enter");
    assert_eq!(
        ui.dialog(),
        Some("AddPriceSource"),
        "the key is still blank"
    );
    ui.press("tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("ApiKey"));
    type_word(&mut ui, "abc");
    assert_eq!(
        ui.dialog_value("api_key").as_deref(),
        Some("\u{2022}\u{2022}\u{2022}")
    );
    assert!(ui.is_drawn("price-source-key-note"));
    ui.press("enter");
    assert_eq!(ui.dialog(), None);

    // Reopened, the source keeps its key parameter but has no key to show.
    ui.right_click("price-source-row-0");
    ui.click("settings-units-menu-edit-source");
    assert_eq!(ui.dialog_value("key_param").as_deref(), Some("apikey"));
    assert_eq!(ui.dialog_value("api_key").as_deref(), Some(""));
}

#[gpui::test]
fn escape_discards_an_add_price_source_draft(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    fill_custom_source(&mut ui, "broker");

    ui.press("escape");
    assert_eq!(ui.dialog(), None);
    assert_eq!(source_names(&mut ui).len(), 4);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn a_built_in_edit_takes_its_name_and_updates_by_key(app: &mut TestAppContext) {
    let mut ui = on_units(app);

    ui.right_click("price-source-row-3");
    ui.click("settings-units-menu-edit-source");
    assert_eq!(ui.dialog(), Some("EditBuiltInPriceSource"));
    for _ in "Yahoo Finance".chars() {
        ui.press("backspace");
    }
    type_word(&mut ui, "alpha");
    ui.press("tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Updates"));
    ui.press("x");
    assert_eq!(
        ui.dialog_value("name").as_deref(),
        Some("alpha"),
        "Updates swallows typing"
    );
    ui.press("right");
    assert_eq!(ui.dialog_value("updates").as_deref(), Some("Daily"));
    ui.press("tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Name"));
    ui.press("enter");

    assert_eq!(ui.dialog(), None);
    assert_eq!(
        source_names(&mut ui),
        vec!["alpha", "CoinGecko", "Frankfurter", "Manual entry"]
    );
}

#[gpui::test]
fn a_built_in_edit_refuses_a_blank_name(app: &mut TestAppContext) {
    let mut ui = on_units(app);

    ui.double_click("price-source-row-1");
    for _ in "Frankfurter".chars() {
        ui.press("backspace");
    }
    ui.press("enter");
    assert_eq!(ui.dialog(), Some("EditBuiltInPriceSource"));
    ui.press("escape");
    assert_eq!(source_names(&mut ui)[1], "Frankfurter");
}

#[gpui::test]
fn delete_price_source_enter_refuses_until_the_name_is_typed_back(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    fill_custom_source(&mut ui, "broker");
    ui.press("enter");

    ui.right_click("price-source-row-0");
    ui.click("settings-units-menu-delete-source");
    assert_eq!(ui.dialog(), Some("DeletePriceSource"));
    type_word(&mut ui, "Broker");
    ui.press("enter");
    assert_eq!(
        ui.dialog(),
        Some("DeletePriceSource"),
        "the match is case-sensitive"
    );
    for _ in 0..6 {
        ui.press("backspace");
    }
    type_word(&mut ui, "broker");
    assert_eq!(ui.dialog_value("confirm").as_deref(), Some("broker"));
    ui.press("enter");

    assert_eq!(ui.dialog(), None);
    assert_eq!(source_names(&mut ui).len(), 4);
}

#[gpui::test]
fn escape_closes_delete_price_source_without_a_write(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    fill_custom_source(&mut ui, "broker");
    ui.press("enter");

    ui.right_click("price-source-row-0");
    ui.click("settings-units-menu-delete-source");
    type_word(&mut ui, "broker");
    ui.press("escape");
    assert_eq!(ui.dialog(), None);
    assert_eq!(source_names(&mut ui)[0], "broker");
}

// ---------------------------------------------------------------------------------------------
// One selection across both tables (#714)
// ---------------------------------------------------------------------------------------------

/// The page's selection as `(unit code, price source name)`: one of the two at a time.
fn selection(ui: &mut Harness<'_>) -> (Option<String>, Option<String>) {
    let page = ui.settings_units();
    (page.selected_code, page.selected_source)
}

fn unit(code: &str) -> (Option<String>, Option<String>) {
    (Some(code.to_string()), None)
}

fn source(name: &str) -> (Option<String>, Option<String>) {
    (None, Some(name.to_string()))
}

#[gpui::test]
fn at_large_j_and_k_walk_the_units_then_the_price_sources_as_one_list(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.press("l");
    assert_eq!(selection(&mut ui), unit("aud"));

    ui.press("j j");
    assert_eq!(selection(&mut ui), unit("vas"));
    ui.press("j");
    assert_eq!(
        selection(&mut ui),
        source("CoinGecko"),
        "j leaves the last unit for the first source"
    );
    ui.press("k");
    assert_eq!(selection(&mut ui), unit("vas"));
    ui.press("G");
    assert_eq!(selection(&mut ui), source("Yahoo Finance"));
    ui.press("g g");
    assert_eq!(selection(&mut ui), unit("aud"));
    ui.press("ctrl-d");
    assert_eq!(
        selection(&mut ui),
        source("Manual entry"),
        "a half page is five rows down the one list"
    );
    ui.press("ctrl-u");
    assert_eq!(selection(&mut ui), unit("aud"));
}

#[gpui::test]
fn tab_jumps_between_the_tables_and_returns_to_each_tables_last_row(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.press("l j");
    assert_eq!(selection(&mut ui), unit("btc"));

    ui.press("tab");
    assert_eq!(
        selection(&mut ui),
        source("CoinGecko"),
        "a table not yet visited opens on its first row"
    );
    ui.press("j");
    assert_eq!(selection(&mut ui), source("Frankfurter"));
    ui.press("tab");
    assert_eq!(selection(&mut ui), unit("btc"));
    ui.press("tab");
    assert_eq!(selection(&mut ui), source("Frankfurter"));
}

#[gpui::test]
fn shift_tab_still_cycles_the_focus_zones(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.press("l");
    ui.press("shift-tab");
    assert_ne!(
        ui.read(|shell| shell.nav().focus()),
        bin_desktop::navigation::nav::FocusZone::View
    );
    assert_eq!(selection(&mut ui), unit("aud"));
}

fn status_message(ui: &mut Harness<'_>) -> Option<String> {
    ui.read(|shell| shell.status_message().map(str::to_string))
}

#[gpui::test]
fn on_a_built_in_source_n_adds_e_edits_t_is_not_built_and_d_does_nothing(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.press("l tab");
    assert_eq!(selection(&mut ui), source("CoinGecko"));

    ui.press("n");
    assert_eq!(ui.dialog(), Some("AddPriceSource"));
    ui.press("escape");
    ui.press("e");
    assert_eq!(ui.dialog(), Some("EditBuiltInPriceSource"));
    assert_eq!(ui.dialog_value("name").as_deref(), Some("CoinGecko"));
    ui.press("escape");
    ui.press("d");
    assert_eq!(ui.dialog(), None, "a built-in is never deleted");
    assert_eq!(status_message(&mut ui), None);
    ui.press("t");
    assert!(
        status_message(&mut ui).is_some(),
        "testing is not built yet"
    );
}

#[gpui::test]
fn on_a_custom_source_d_opens_delete_price_source(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    fill_custom_source(&mut ui, "broker");
    ui.press("enter");
    ui.press("l tab");
    assert_eq!(selection(&mut ui), source("broker"));

    ui.press("d");
    assert_eq!(ui.dialog(), Some("DeletePriceSource"));
    ui.press("escape");
    ui.press("e");
    assert_eq!(ui.dialog(), Some("EditPriceSource"));
}

#[gpui::test]
fn manual_entry_ignores_e_d_and_t(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.press("l tab j j");
    assert_eq!(selection(&mut ui), source("Manual entry"));

    ui.press("e d t");
    assert_eq!(ui.dialog(), None);
    assert_eq!(status_message(&mut ui), None);
    ui.press("n");
    assert_eq!(ui.dialog(), Some("AddPriceSource"));
}

#[gpui::test]
fn at_x_large_the_pane_follows_the_selection_into_either_table(app: &mut TestAppContext) {
    let mut ui = on_units_xlarge(app);
    assert!(
        !ui.is_drawn("units-pane-edit-source"),
        "aud has no price source"
    );
    ui.press("j");
    assert!(ui.is_drawn("units-pane-edit-source"), "btc's CoinGecko");
    ui.press("tab j j");
    assert_eq!(selection(&mut ui), source("Manual entry"));
    ui.press("G");
    assert_eq!(selection(&mut ui), source("Yahoo Finance"));
    ui.press("e");
    assert_eq!(ui.dialog(), Some("EditBuiltInPriceSource"));
}

fn hints(ui: &mut Harness<'_>) -> Vec<(String, String)> {
    let hints = ui.status().map(|status| status.hints).unwrap_or_default();
    hints.into_iter().take(6).collect()
}

fn pairs(expected: &[(&str, &str)]) -> Vec<(String, String)> {
    expected
        .iter()
        .map(|(key, action)| (key.to_string(), action.to_string()))
        .collect()
}

#[gpui::test]
fn the_status_line_follows_the_selections_table(app: &mut TestAppContext) {
    let unit_hints = pairs(&[
        ("j/k", "unit"),
        ("e", "edit"),
        ("d", "delete"),
        ("n", "new"),
        ("t", "test source"),
        ("tab", "price sources"),
    ]);
    let source_hints = pairs(&[
        ("j/k", "price source"),
        ("e", "edit"),
        ("d", "delete"),
        ("n", "new"),
        ("t", "test"),
        ("tab", "units"),
    ]);
    let mut ui = on_units(app);
    ui.press("l");
    assert_eq!(hints(&mut ui), unit_hints);
    ui.press("tab");
    assert_eq!(hints(&mut ui), source_hints);

    ui.resize_to(WindowSize::XLarge);
    assert_eq!(hints(&mut ui), source_hints);
    ui.press("tab");
    assert_eq!(hints(&mut ui), unit_hints);
}

// ---------------------------------------------------------------------------------------------
// The X-Large pane's PRICE SOURCE and VALUES groups (DESKTOP-125, DESKTOP-140, #716)
// ---------------------------------------------------------------------------------------------

/// The pane's PRICE SOURCE fact rows' labels.
fn fact_labels(ui: &mut Harness<'_>) -> Vec<String> {
    ui.settings_units()
        .source_facts
        .into_iter()
        .map(|(label, _)| label)
        .collect()
}

/// The pane's groups and PRICE SOURCE buttons, as drawn: `is_drawn` can't say a selector has
/// gone, since gpui keeps a frame's debug bounds once drawn.
fn pane(ui: &mut Harness<'_>) -> (Vec<String>, Vec<String>) {
    let page = ui.settings_units();
    (page.pane_groups, page.pane_source_buttons)
}

fn strings(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| item.to_string()).collect()
}

fn fact(ui: &mut Harness<'_>, label: &str) -> Option<String> {
    ui.settings_units()
        .source_facts
        .into_iter()
        .find_map(|(fact, value)| (fact == label).then_some(value))
}

#[gpui::test]
fn at_x_large_a_selected_units_source_group_reads_its_link(app: &mut TestAppContext) {
    let mut ui = on_units_xlarge(app);
    assert!(fact_labels(&mut ui).is_empty(), "aud sits on Manual entry");
    assert_eq!(pane(&mut ui).1, strings(&["add-source"]));

    ui.press("j");
    assert_eq!(
        fact_labels(&mut ui),
        ["Source", "URL", "Updates", "Last price", "Last updated"]
    );
    assert_eq!(fact(&mut ui, "Source").as_deref(), Some("CoinGecko"));
    assert_eq!(
        fact(&mut ui, "URL").as_deref(),
        Some("https://api.coingecko.com/api/v3")
    );
    assert_eq!(fact(&mut ui, "Updates").as_deref(), Some("Every 15 min"));
    assert_eq!(
        fact(&mut ui, "Last price").as_deref(),
        Some("148,212.40 aud")
    );
    assert_eq!(
        pane(&mut ui),
        (
            strings(&["unit", "source", "accounts", "activity"]),
            strings(&["test-source", "edit-source"])
        ),
        "a built-in is not deleted"
    );
}

#[gpui::test]
fn at_x_large_a_units_accounts_are_valued_by_its_last_price(app: &mut TestAppContext) {
    let mut ui = on_units_xlarge(app);
    assert!(
        ui.settings_units().account_values.is_empty(),
        "aud has no price to convert by"
    );
    ui.press("j");
    let values = ui.settings_units().account_values;
    assert!(
        !values.is_empty(),
        "btc's accounts take its CoinGecko price"
    );
    assert!(values.iter().all(|value| value.ends_with(" aud")));
}

#[gpui::test]
fn at_x_large_a_selected_built_in_shows_its_endpoint_and_the_units_it_values(
    app: &mut TestAppContext,
) {
    let mut ui = on_units_xlarge(app);
    ui.press("tab");
    assert_eq!(selection(&mut ui), source("CoinGecko"));

    assert_eq!(
        pane(&mut ui),
        (
            strings(&["source", "values", "activity"]),
            strings(&["test-source", "edit-source"])
        )
    );
    assert!(ui.is_drawn("units-pane-values"));

    let page = ui.settings_units();
    assert_eq!(
        page.source_meta.as_deref(),
        Some("built-in · prices Crypto · key not needed")
    );
    assert_eq!(
        fact_labels(&mut ui),
        [
            "Source",
            "Address",
            "Request",
            "Price path",
            "Updates",
            "Last updated"
        ]
    );
    assert_eq!(
        fact(&mut ui, "Source").as_deref(),
        Some("CoinGecko · built-in")
    );
    assert_eq!(
        fact(&mut ui, "Request").as_deref(),
        Some("/simple/price?ids={symbol}&vs_currencies={quote}")
    );
    assert_eq!(
        page.source_note.as_deref(),
        Some("Built-in sources update with the app, so their address isn't edited here.")
    );
    assert_eq!(
        page.source_values,
        vec![(
            "btc · Bitcoin".to_string(),
            "symbol bitcoin · quoted in aud".to_string(),
            "148,212.40 aud".to_string()
        )]
    );
}

#[gpui::test]
fn at_x_large_a_source_no_unit_uses_says_so_and_has_not_fetched(app: &mut TestAppContext) {
    let mut ui = on_units_xlarge(app);
    ui.press("tab j");
    assert_eq!(selection(&mut ui), source("Frankfurter"));
    assert!(ui.settings_units().source_values.is_empty());
    assert_eq!(
        fact(&mut ui, "Last updated").as_deref(),
        Some("\u{2717} Not fetched yet")
    );
}

#[gpui::test]
fn at_x_large_manual_entry_shows_its_note_and_the_units_priced_by_hand(app: &mut TestAppContext) {
    let mut ui = on_units_xlarge(app);
    ui.press("tab j j");
    assert_eq!(selection(&mut ui), source("Manual entry"));

    let page = ui.settings_units();
    assert_eq!(page.source_meta, None);
    assert!(page.source_facts.is_empty());
    assert_eq!(
        page.source_note.as_deref(),
        Some("Prices you enter by hand.")
    );
    let priced: Vec<&str> = page
        .source_values
        .iter()
        .map(|(unit, _, _)| unit.as_str())
        .collect();
    assert_eq!(
        priced,
        ["aud · Australian Dollar", "vas · Vanguard Aus Shares"]
    );
    assert_eq!(
        pane(&mut ui),
        (strings(&["source", "values", "activity"]), Vec::new()),
        "Manual entry is neither tested, edited nor deleted"
    );
}

#[gpui::test]
fn at_x_large_a_custom_source_shows_its_key_parameter_but_never_the_key(app: &mut TestAppContext) {
    let mut ui = on_units_xlarge(app);
    fill_custom_source(&mut ui, "broker");
    ui.press("tab right tab");
    type_word(&mut ui, "apikey");
    ui.press("tab");
    type_word(&mut ui, "secret");
    ui.press("enter");
    assert_eq!(ui.dialog(), None);
    ui.press("tab");
    assert_eq!(selection(&mut ui), source("broker"));

    let page = ui.settings_units();
    assert_eq!(
        page.source_meta.as_deref(),
        Some("custom · prices every kind · key needed")
    );
    assert_eq!(fact(&mut ui, "Source").as_deref(), Some("broker · custom"));
    assert_eq!(fact(&mut ui, "Authentication").as_deref(), Some("API key"));
    assert_eq!(fact(&mut ui, "Key parameter").as_deref(), Some("apikey"));
    assert!(
        page.source_facts
            .iter()
            .all(|(_, value)| !value.contains("secret")),
        "the key is never shown"
    );
    assert_eq!(page.source_note, None, "no built-in note");
    assert_eq!(
        page.pane_source_buttons,
        strings(&["test-source", "edit-source", "delete-source"])
    );
}

#[gpui::test]
fn add_unit_walks_type_and_source_by_key_and_saves_the_link(app: &mut TestAppContext) {
    let mut ui = on_units(app);

    ui.click("settings-add-unit");
    assert_eq!(
        ui.dialog_value("quote").as_deref(),
        Some("aud (base unit)"),
        "Quoted in defaults to the base unit"
    );
    ui.press("e t h");
    // Type: down steps the closed select straight through the Kinds to Crypto.
    ui.press("tab down");
    assert_eq!(ui.dialog_value("kind").as_deref(), Some("Crypto"));
    ui.press("tab e t h e r");
    ui.press("tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Source"));
    assert_eq!(
        ui.dialog_value("sources").as_deref(),
        Some("CoinGecko|Manual entry")
    );
    // Enter opens the list on Manual entry, down wraps to CoinGecko and enter commits it.
    ui.press("enter down enter");
    assert_eq!(ui.dialog_value("source").as_deref(), Some("CoinGecko"));
    assert_eq!(ui.dialog_value("manual").as_deref(), Some("false"));

    ui.press("tab b i t");
    assert_eq!(ui.dialog_value("symbol").as_deref(), Some("bit"));
    ui.press("enter");
    assert_eq!(ui.dialog(), None);
    let page = ui.settings_units();
    assert_eq!(
        page.unit_links.last(),
        Some(&(
            "eth".to_string(),
            "CoinGecko".to_string(),
            "bit".to_string(),
            "aud".to_string()
        ))
    );
    assert_eq!(
        page.unit_rows.last().map(|row| row.2.as_str()),
        Some("crypto")
    );
}

#[gpui::test]
fn changing_type_refilters_source_and_falls_back_to_manual_entry(app: &mut TestAppContext) {
    let mut ui = on_units(app);

    ui.click("settings-add-unit");
    ui.press("tab down");
    assert_eq!(ui.dialog_value("kind").as_deref(), Some("Crypto"));
    ui.press("tab tab enter down enter");
    assert_eq!(ui.dialog_value("source").as_deref(), Some("CoinGecko"));

    // Back to Type and down to Equity: CoinGecko no longer prices it.
    ui.press("shift-tab shift-tab down");
    assert_eq!(ui.dialog_value("kind").as_deref(), Some("Equity"));
    assert_eq!(ui.dialog_value("source").as_deref(), Some("Manual entry"));
    assert_eq!(
        ui.dialog_value("sources").as_deref(),
        Some("Manual entry|Yahoo Finance")
    );
}

#[gpui::test]
fn manual_entry_hides_symbol_and_quote_and_tab_skips_them(app: &mut TestAppContext) {
    let mut ui = on_units(app);

    ui.click("settings-add-unit");
    assert_eq!(ui.dialog_value("manual").as_deref(), Some("true"));
    assert_eq!(
        ui.dialog_value("visible").as_deref(),
        Some("Code|Type|Name|Source")
    );
    ui.press("tab tab tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Source"));
    ui.press("tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Code"));
}

#[gpui::test]
fn edit_unit_keeps_the_stored_link_and_saves_a_changed_symbol(app: &mut TestAppContext) {
    let mut ui = on_units(app);

    row_action(&mut ui, "unit-row-1", "edit-unit");
    assert_eq!(ui.dialog_value("source").as_deref(), Some("CoinGecko"));
    assert_eq!(ui.dialog_value("manual").as_deref(), Some("false"));
    ui.press("tab tab tab tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("Symbol"));
    ui.press("backspace");
    ui.press("enter");
    assert_eq!(ui.dialog(), None);
    let link = ui.settings_units().unit_links[1].clone();
    assert_eq!(link.1, "CoinGecko");
    assert_eq!(link.2, "bitcoi");
}

#[gpui::test]
fn the_base_glyph_follows_the_ledgers_base_unit(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    assert_eq!(ui.settings_units().base_code.as_deref(), Some("aud"));

    ui.press("escape");
    ui.press("g g");
    ui.press("l");
    ui.press("j");
    ui.press("escape escape");
    ui.press("j j");
    assert_eq!(ui.settings().page, "Units");
    assert_eq!(ui.settings_units().base_code.as_deref(), Some("btc"));
}

#[gpui::test]
fn deleting_the_base_unit_moves_the_base_with_up_and_down(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    row_action(&mut ui, "unit-row-0", "delete-unit");
    let options = ui.dialog_value("new_base_options").unwrap_or_default();
    let options: Vec<&str> = options.split('|').collect();
    assert_eq!(
        ui.dialog_value("new_base").as_deref(),
        Some(options[0]),
        "the first Unit that stays starts chosen"
    );

    ui.press("down");
    assert_eq!(ui.dialog_value("new_base").as_deref(), Some(options[1]));
    let chosen = ui.dialog_value("new_base").expect("a chosen base");

    for ch in "aud".chars() {
        ui.press(&ch.to_string());
    }
    ui.press("enter");
    assert_eq!(ui.dialog(), None);
    let base = ui.settings_units().base_code.expect("a base Unit");
    assert!(chosen.starts_with(&base));
}
