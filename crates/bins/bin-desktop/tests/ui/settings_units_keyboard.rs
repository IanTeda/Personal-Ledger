//! The Settings Units page driven by keys: reaching the page down the index, and the Add, Edit
//! and Delete unit and price source dialogs' keys once open: typing, `tab` between fields,
//! `left`/`right` on a segmented control, `enter` to save or confirm, and `escape` to cancel. Below
//! X-Large the page has no keys of its own, and a Price Sources row takes no selection yet, so each
//! dialog is opened with the square + button or a row's context menu. State is read back from `Shell`.

use super::common;

use bin_desktop::{
    WindowSize,
    navigation::nav::{InputMode, Noun},
};
use common::Harness;
use gpui::TestAppContext;

/// Below X-Large the page keeps no selected row, so the older cases run at Large; the X-Large
/// cases below drive the detail pane.
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
    ui.press("tab tab");
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
    ui.press("a tab b");
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
    ui.press("tab");
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
    ui.press("tab");
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
fn below_x_large_the_page_keeps_no_keys_of_its_own(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::Large);
    ui.press("g s");
    ui.press("j j");
    ui.press("l");
    assert!(!ui.is_drawn("units-detail-pane"));
    ui.press("n");
    assert_eq!(ui.dialog(), None);
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
