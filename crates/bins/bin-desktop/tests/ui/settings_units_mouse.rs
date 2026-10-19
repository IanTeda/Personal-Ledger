//! The Settings Units page driven by clicks: the index entry, the Add unit button, the row edit
//! and delete buttons, the Add, Edit and Delete unit dialogs' Type control, confirm and cancel
//! buttons, and the Price Sources table with its Add, Edit and Delete price source Dialogs. State is
//! read back from `Shell`.

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
    ui.click("settings-index-Units");
    assert_eq!(ui.settings().page, "Units");
    ui
}

fn mode(ui: &mut Harness<'_>) -> InputMode {
    ui.read(|shell| shell.nav().mode())
}

fn status(ui: &mut Harness<'_>) -> Option<String> {
    ui.read(|shell| shell.status_message().map(str::to_string))
}

/// The open row menu: the row it is on and its entries' slugs.
fn menu(ui: &mut Harness<'_>) -> Option<(String, Vec<String>)> {
    ui.settings_units().menu
}

/// Right-clicks the row and picks an entry of its context menu: the rows carry no buttons.
fn row_action(ui: &mut Harness<'_>, row: &str, action: &str) {
    ui.right_click(row);
    ui.click(&format!("settings-units-menu-{action}"));
}

#[gpui::test]
fn clicking_the_units_index_entry_swaps_to_the_page(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("g s");
    ui.click("settings-index-Accounts");
    assert_eq!(ui.settings().page, "Accounts");

    ui.click("settings-index-Units");
    assert_eq!(ui.settings().page, "Units");
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn the_add_unit_button_opens_and_cancel_closes_without_a_write(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let before = ui.settings_units().unit_rows.len();

    ui.click("settings-add-unit");
    assert_eq!(ui.dialog(), Some("AddUnit"));
    assert_eq!(mode(&mut ui), InputMode::Dialog);
    ui.click("add-unit-cancel");
    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.settings_units().unit_rows.len(), before);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn the_add_unit_confirm_button_does_nothing_until_code_and_name_are_filled(
    app: &mut TestAppContext,
) {
    let mut ui = on_units(app);
    let before = ui.settings_units().unit_rows.len();

    ui.click("settings-add-unit");
    ui.click("add-unit-confirm");
    assert_eq!(ui.dialog(), Some("AddUnit"));
    assert_eq!(ui.settings_units().unit_rows.len(), before);
    ui.press("g o l d");
    ui.click("add-unit-confirm");
    assert_eq!(ui.dialog(), Some("AddUnit"));
    assert_eq!(ui.settings_units().unit_rows.len(), before);
}

#[gpui::test]
fn the_add_unit_type_control_and_confirm_button_add_the_unit(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let before = ui.settings_units().unit_rows.len();

    ui.click("settings-add-unit");
    ui.press("g o l d");
    ui.click("add-unit-name");
    ui.press("g o l d");
    ui.click("add-unit-type");
    ui.click("add-unit-type-option-4");
    assert_eq!(ui.dialog_value("kind").as_deref(), Some("Commodity"));
    ui.click("add-unit-confirm");
    let units_page = ui.settings_units();
    assert_eq!(ui.dialog(), None);
    assert_eq!(units_page.unit_rows.len(), before + 1);
    assert_eq!(
        units_page
            .unit_rows
            .last()
            .map(|row| (row.0.as_str(), row.2.as_str())),
        Some(("gold", "commodity"))
    );
}

#[gpui::test]
fn the_edit_unit_button_opens_a_dialog_prefilled_from_the_row(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let (code, name, kind) = ui.settings_units().unit_rows[0].clone();

    row_action(&mut ui, "unit-row-0", "edit-unit");
    assert_eq!(ui.dialog(), Some("EditUnit"));
    assert_eq!(ui.dialog_value("code").as_deref(), Some(code.as_str()));
    assert_eq!(ui.dialog_value("name").as_deref(), Some(name.as_str()));
    ui.click("edit-unit-cancel");
    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.settings_units().unit_rows[0], (code, name, kind));
}

#[gpui::test]
fn the_edit_unit_dialog_prefills_the_stored_type(app: &mut TestAppContext) {
    let mut ui = on_units(app);

    // vas is seeded as an Equity, which the old three-option selector could only show as Custom.
    row_action(&mut ui, "unit-row-2", "edit-unit");
    assert_eq!(ui.dialog_value("kind").as_deref(), Some("Equity"));
}

#[gpui::test]
fn the_edit_unit_dialog_saves_a_changed_type_by_mouse(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let (code, name, _) = ui.settings_units().unit_rows[0].clone();

    row_action(&mut ui, "unit-row-0", "edit-unit");
    ui.click("edit-unit-type");
    ui.click("edit-unit-type-option-1");
    assert_eq!(ui.dialog_value("kind").as_deref(), Some("Crypto"));
    ui.click("edit-unit-confirm");
    let units_page = ui.settings_units();
    assert_eq!(ui.dialog(), None);
    assert_eq!(units_page.unit_rows[0], (code, name, "crypto".to_string()));
}

#[gpui::test]
fn the_delete_unit_button_opens_a_dialog_and_cancel_closes_without_a_write(
    app: &mut TestAppContext,
) {
    let mut ui = on_units(app);
    let before = ui.settings_units().unit_codes.clone();

    row_action(&mut ui, "unit-row-0", "delete-unit");
    assert_eq!(ui.dialog(), Some("DeleteUnit"));
    ui.click("delete-unit-cancel");
    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.settings_units().unit_codes, before);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn the_delete_unit_confirm_button_refuses_until_the_code_is_typed_back(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let before = ui.settings_units().unit_codes.clone();
    let code = before[0].clone();

    row_action(&mut ui, "unit-row-0", "delete-unit");
    ui.click("delete-unit-confirm");
    assert_eq!(ui.dialog(), Some("DeleteUnit"));
    assert_eq!(ui.settings_units().unit_codes, before);

    ui.press(
        &code
            .chars()
            .next()
            .expect("seeded units have codes")
            .to_string(),
    );
    ui.click("delete-unit-confirm");
    assert_eq!(ui.dialog(), Some("DeleteUnit"));
    assert_eq!(ui.settings_units().unit_codes, before);
}

#[gpui::test]
fn the_delete_unit_confirm_button_removes_the_row_once_the_code_matches(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let before = ui.settings_units().unit_codes.clone();
    let code = before[0].clone();

    row_action(&mut ui, "unit-row-0", "delete-unit");
    for ch in code.chars() {
        ui.press(&ch.to_string());
    }
    ui.click("delete-unit-confirm");
    let after = ui.settings_units().unit_codes;
    assert_eq!(ui.dialog(), None);
    assert_eq!(after.len(), before.len() - 1);
    assert!(!after.contains(&code));
}

fn on_units_xlarge(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.resize_to(WindowSize::XLarge);
    ui.press("g s");
    ui.click("settings-index-Units");
    assert_eq!(ui.settings().page, "Units");
    ui
}

#[gpui::test]
fn at_x_large_the_pane_docks_with_its_groups(app: &mut TestAppContext) {
    let mut ui = on_units_xlarge(app);
    assert!(ui.is_drawn("units-detail-pane"));
    assert!(ui.is_drawn("units-pane-unit"));
    assert!(ui.is_drawn("units-pane-source"));
    assert!(ui.is_drawn("units-pane-accounts"));
    assert!(ui.is_drawn("units-pane-activity"));
}

#[gpui::test]
fn at_x_large_clicking_a_row_selects_it_and_focuses_the_page(app: &mut TestAppContext) {
    let mut ui = on_units_xlarge(app);
    assert!(!ui.settings().page_focused);

    ui.click("unit-row-1");
    assert_eq!(ui.settings_units().selected_code.as_deref(), Some("btc"));
    assert!(ui.settings().page_focused);
}

#[gpui::test]
fn at_x_large_the_pane_buttons_open_the_dialogs_for_the_selected_unit(app: &mut TestAppContext) {
    let mut ui = on_units_xlarge(app);
    ui.click("unit-row-1");

    ui.click("units-pane-edit-unit");
    assert_eq!(ui.dialog(), Some("EditUnit"));
    ui.press("escape");

    ui.click("units-pane-delete-unit");
    assert_eq!(ui.dialog(), Some("DeleteUnit"));
    ui.click("delete-unit-cancel");
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn at_x_large_a_unit_without_a_source_offers_to_add_one(app: &mut TestAppContext) {
    let mut ui = on_units_xlarge(app);
    assert!(ui.is_drawn("units-pane-add-source"));
    assert!(!ui.is_drawn("units-pane-test-source"));

    ui.click("unit-row-1");
    assert!(ui.is_drawn("units-pane-test-source"));
    ui.click("units-pane-test-source");
    assert!(status(&mut ui).is_some());
}

#[gpui::test]
fn at_x_large_a_confirmed_delete_lands_in_activity_and_raises_no_toast(app: &mut TestAppContext) {
    let mut ui = on_units_xlarge(app);
    ui.click("unit-row-1");
    ui.click("units-pane-delete-unit");
    for ch in "btc".chars() {
        ui.press(&ch.to_string());
    }
    ui.click("delete-unit-confirm");

    assert_eq!(
        ui.settings_units().activity,
        vec!["Deleted unit btc".to_string()]
    );
    assert!(ui.toasts().visible.is_empty());
}

#[gpui::test]
fn below_x_large_a_confirmed_delete_still_raises_its_toast(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    row_action(&mut ui, "unit-row-1", "delete-unit");
    for ch in "btc".chars() {
        ui.press(&ch.to_string());
    }
    ui.click("delete-unit-confirm");

    assert!(ui.settings_units().activity.is_empty());
    assert_eq!(ui.toasts().visible.len(), 1);
}

#[gpui::test]
fn rows_carry_no_buttons_at_any_size(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    for selector in [
        "unit-edit-0",
        "unit-delete-0",
        "price-source-test-0",
        "price-source-edit-0",
        "price-source-delete-0",
    ] {
        assert!(!ui.is_drawn(selector), "{selector} should not be drawn");
    }
    assert!(ui.is_drawn("settings-add-unit"));
    assert!(ui.is_drawn("settings-add-price-source"));
}

#[gpui::test]
fn double_clicking_a_unit_row_opens_its_edit_dialog(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let (code, _, _) = ui.settings_units().unit_rows[2].clone();

    ui.double_click("unit-row-2");
    assert_eq!(ui.dialog(), Some("EditUnit"));
    assert_eq!(ui.dialog_value("code").as_deref(), Some(code.as_str()));
}

#[gpui::test]
fn right_clicking_a_unit_row_opens_a_menu_with_edit_then_delete(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    assert_eq!(menu(&mut ui), None);

    ui.right_click("unit-row-1");
    assert_eq!(
        menu(&mut ui),
        Some((
            "unit:btc".to_string(),
            vec!["edit-unit".to_string(), "delete-unit".to_string()]
        ))
    );
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn a_menu_entry_closes_the_menu_and_runs_its_action(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let (code, _, _) = ui.settings_units().unit_rows[1].clone();

    row_action(&mut ui, "unit-row-1", "edit-unit");
    assert_eq!(menu(&mut ui), None);
    assert_eq!(ui.dialog(), Some("EditUnit"));
    assert_eq!(ui.dialog_value("code").as_deref(), Some(code.as_str()));
}

#[gpui::test]
fn a_press_outside_the_menu_closes_it(app: &mut TestAppContext) {
    let mut ui = on_units(app);

    ui.right_click("unit-row-2");
    assert!(menu(&mut ui).is_some());
    ui.click("unit-row-0");
    assert_eq!(menu(&mut ui), None);
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn at_x_large_the_accounts_group_lists_the_selected_units_accounts(app: &mut TestAppContext) {
    let mut ui = on_units_xlarge(app);

    assert!(ui.is_drawn("units-pane-accounts"));
}

// ---------------------------------------------------------------------------------------------
// Price Sources (16g, 16z·custom, 16z·edit, 16z·del)
// ---------------------------------------------------------------------------------------------

/// The seeded rows, alphabetical: CoinGecko, Frankfurter, Manual entry, Yahoo Finance.
fn source_names(ui: &mut Harness<'_>) -> Vec<String> {
    ui.settings_units()
        .source_rows
        .into_iter()
        .map(|(name, _, _)| name)
        .collect()
}

fn type_word(ui: &mut Harness<'_>, word: &str) {
    let keys: Vec<String> = word.chars().map(|ch| ch.to_string()).collect();
    ui.press(&keys.join(" "));
}

/// Adds a Custom source named `name` by clicking into each required field.
fn add_custom_source(ui: &mut Harness<'_>, name: &str) {
    ui.click("settings-add-price-source");
    type_word(ui, name);
    for field in [
        "price-source-address",
        "price-source-request",
        "price-source-price-path",
    ] {
        ui.click(field);
        type_word(ui, "x");
    }
    ui.click("add-price-source-confirm");
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn the_price_sources_table_lists_the_seeded_sources_alphabetically(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let rows = ui.settings_units().source_rows;
    let names: Vec<&str> = rows.iter().map(|row| row.0.as_str()).collect();
    assert_eq!(
        names,
        vec!["CoinGecko", "Frankfurter", "Manual entry", "Yahoo Finance"]
    );
    assert_eq!(rows[0].1, "https://api.coingecko.com/api/v3");
    assert!(
        !rows[0].2.starts_with('\u{2717}') && rows[0].2 != "\u{2014}",
        "CoinGecko has fetched: {}",
        rows[0].2
    );
    assert_eq!(rows[1].2, "\u{2717} Not fetched yet");
    assert_eq!(
        rows[2],
        (
            "Manual entry".to_string(),
            "\u{2014}".to_string(),
            "\u{2014}".to_string()
        )
    );
}

#[gpui::test]
fn a_built_ins_menu_offers_test_and_edit_but_no_delete(app: &mut TestAppContext) {
    let mut ui = on_units(app);

    ui.right_click("price-source-row-0");
    assert_eq!(
        menu(&mut ui),
        Some((
            "source:CoinGecko".to_string(),
            vec!["test-source".to_string(), "edit-source".to_string()]
        ))
    );
}

#[gpui::test]
fn manual_entry_opens_no_menu_and_no_dialog(app: &mut TestAppContext) {
    let mut ui = on_units(app);

    ui.right_click("price-source-row-2");
    assert_eq!(menu(&mut ui), None);
    ui.double_click("price-source-row-2");
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn test_price_source_is_not_yet_built(app: &mut TestAppContext) {
    let mut ui = on_units(app);

    row_action(&mut ui, "price-source-row-0", "test-source");
    assert_eq!(
        status(&mut ui).as_deref(),
        Some("test price source \u{2014} not yet built")
    );
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn the_add_price_source_button_opens_the_custom_form_and_cancel_writes_nothing(
    app: &mut TestAppContext,
) {
    let mut ui = on_units(app);

    ui.click("settings-add-price-source");
    assert_eq!(ui.dialog(), Some("AddPriceSource"));
    assert!(ui.is_drawn("price-source-key-note"));
    assert!(!ui.is_drawn("price-source-api-key"));
    ui.click("add-price-source-confirm");
    assert_eq!(
        ui.dialog(),
        Some("AddPriceSource"),
        "an empty form can't add"
    );
    ui.click("add-price-source-cancel");
    assert_eq!(ui.dialog(), None);
    assert_eq!(source_names(&mut ui).len(), 4);
}

#[gpui::test]
fn adding_a_custom_source_by_mouse_lists_it_in_order_and_toasts(app: &mut TestAppContext) {
    let mut ui = on_units(app);

    ui.click("settings-add-price-source");
    type_word(&mut ui, "broker");
    ui.click("price-source-updates-1");
    assert_eq!(ui.dialog_value("updates").as_deref(), Some("Hourly"));
    ui.click("price-source-auth-1");
    assert!(ui.is_drawn("price-source-api-key"));
    for field in [
        "price-source-address",
        "price-source-request",
        "price-source-price-path",
        "price-source-key-param",
    ] {
        ui.click(field);
        type_word(&mut ui, "x");
    }
    ui.click("add-price-source-confirm");
    assert_eq!(
        ui.dialog(),
        Some("AddPriceSource"),
        "API key authentication needs the key typed"
    );
    ui.click("price-source-api-key");
    type_word(&mut ui, "secret");
    assert_eq!(
        ui.dialog_value("api_key").as_deref(),
        Some("\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}"),
        "the key is masked"
    );
    ui.click("add-price-source-confirm");

    assert_eq!(ui.dialog(), None);
    assert_eq!(
        source_names(&mut ui),
        vec![
            "broker",
            "CoinGecko",
            "Frankfurter",
            "Manual entry",
            "Yahoo Finance"
        ]
    );
    assert_eq!(
        ui.settings_units().source_rows[0].2,
        "\u{2717} Not fetched yet"
    );
    assert_eq!(ui.toasts().visible.len(), 1);
}

#[gpui::test]
fn double_clicking_a_built_in_opens_its_edit_and_a_rename_re_sorts(app: &mut TestAppContext) {
    let mut ui = on_units(app);

    ui.double_click("price-source-row-0");
    assert_eq!(ui.dialog(), Some("EditBuiltInPriceSource"));
    assert_eq!(ui.dialog_value("name").as_deref(), Some("CoinGecko"));
    assert!(ui.is_drawn("edit-built-in-source-facts"));
    assert!(ui.is_drawn("edit-built-in-source-usage"));
    ui.click("edit-built-in-source-updates-2");
    assert_eq!(ui.dialog_value("updates").as_deref(), Some("Daily"));
    ui.click("edit-built-in-source-name");
    for _ in "CoinGecko".chars() {
        ui.press("backspace");
    }
    type_word(&mut ui, "zeta");
    ui.click("edit-built-in-source-confirm");

    assert_eq!(ui.dialog(), None);
    assert_eq!(
        source_names(&mut ui),
        vec!["Frankfurter", "Manual entry", "Yahoo Finance", "zeta"]
    );
}

#[gpui::test]
fn edit_price_source_from_the_menu_cancels_without_a_write(app: &mut TestAppContext) {
    let mut ui = on_units(app);

    row_action(&mut ui, "price-source-row-1", "edit-source");
    assert_eq!(ui.dialog(), Some("EditBuiltInPriceSource"));
    ui.press("backspace");
    ui.click("edit-built-in-source-cancel");
    assert_eq!(ui.dialog(), None);
    assert_eq!(source_names(&mut ui)[1], "Frankfurter");
}

#[gpui::test]
fn a_custom_sources_edit_opens_its_fields_prefilled_and_saves(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    add_custom_source(&mut ui, "broker");

    ui.right_click("price-source-row-0");
    assert_eq!(
        menu(&mut ui).map(|(_, actions)| actions),
        Some(vec![
            "test-source".to_string(),
            "edit-source".to_string(),
            "delete-source".to_string()
        ])
    );
    ui.click("settings-units-menu-edit-source");
    assert_eq!(ui.dialog(), Some("EditPriceSource"));
    assert_eq!(ui.dialog_value("name").as_deref(), Some("broker"));
    assert_eq!(ui.dialog_value("address").as_deref(), Some("x"));
    type_word(&mut ui, "s");
    ui.click("edit-price-source-confirm");

    assert_eq!(ui.dialog(), None);
    assert_eq!(source_names(&mut ui)[0], "brokers");
}

#[gpui::test]
fn deleting_a_custom_source_needs_its_exact_name_typed_back(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    add_custom_source(&mut ui, "broker");

    row_action(&mut ui, "price-source-row-0", "delete-source");
    assert_eq!(ui.dialog(), Some("DeletePriceSource"));
    type_word(&mut ui, "broke");
    ui.click("delete-price-source-confirm");
    assert_eq!(ui.dialog(), Some("DeletePriceSource"));
    type_word(&mut ui, "r");
    ui.click("delete-price-source-confirm");

    assert_eq!(ui.dialog(), None);
    assert_eq!(
        source_names(&mut ui),
        vec!["CoinGecko", "Frankfurter", "Manual entry", "Yahoo Finance"]
    );
}

#[gpui::test]
fn delete_price_source_cancel_keeps_the_source(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    add_custom_source(&mut ui, "broker");

    row_action(&mut ui, "price-source-row-0", "delete-source");
    ui.click("delete-price-source-cancel");
    assert_eq!(ui.dialog(), None);
    assert_eq!(source_names(&mut ui)[0], "broker");
}

#[gpui::test]
fn at_x_large_a_saved_price_source_lands_in_activity_without_a_toast(app: &mut TestAppContext) {
    let mut ui = on_units_xlarge(app);
    add_custom_source(&mut ui, "broker");

    assert_eq!(
        ui.settings_units().activity,
        vec!["Saved price source broker".to_string()]
    );
    assert!(ui.toasts().visible.is_empty());
}

#[gpui::test]
fn at_x_large_the_pane_opens_the_selected_units_source_dialogs(app: &mut TestAppContext) {
    let mut ui = on_units_xlarge(app);

    ui.click("units-pane-add-source");
    assert_eq!(ui.dialog(), Some("AddPriceSource"));
    ui.click("add-price-source-cancel");

    ui.click("unit-row-1");
    assert!(
        !ui.is_drawn("units-pane-delete-source"),
        "btc's source is a built-in"
    );
    ui.click("units-pane-edit-source");
    assert_eq!(ui.dialog(), Some("EditBuiltInPriceSource"));
    assert_eq!(ui.dialog_value("name").as_deref(), Some("CoinGecko"));
}

// ---------------------------------------------------------------------------------------------
// One selection across both tables (#714)
// ---------------------------------------------------------------------------------------------

/// The page's selection as `(unit code, price source name)`: one of the two at a time.
fn selection(ui: &mut Harness<'_>) -> (Option<String>, Option<String>) {
    let page = ui.settings_units();
    (page.selected_code, page.selected_source)
}

#[gpui::test]
fn at_large_clicking_a_price_sources_row_selects_it_and_focuses_the_page(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    assert!(!ui.settings().page_focused);

    ui.click("price-source-row-1");
    assert_eq!(selection(&mut ui), (None, Some("Frankfurter".to_string())));
    assert!(ui.settings().page_focused);
    ui.click("unit-row-2");
    assert_eq!(selection(&mut ui), (Some("vas".to_string()), None));
}

#[gpui::test]
fn right_clicking_a_price_sources_row_selects_it(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.right_click("price-source-row-0");
    assert_eq!(selection(&mut ui), (None, Some("CoinGecko".to_string())));
    assert!(menu(&mut ui).is_some());
}

#[gpui::test]
fn at_x_large_a_clicked_source_is_where_the_keys_carry_on_from(app: &mut TestAppContext) {
    let mut ui = on_units_xlarge(app);
    ui.click("unit-row-1");
    ui.click("price-source-row-1");
    assert_eq!(selection(&mut ui), (None, Some("Frankfurter".to_string())));

    ui.press("j");
    assert_eq!(selection(&mut ui), (None, Some("Manual entry".to_string())));
    ui.press("tab");
    assert_eq!(
        selection(&mut ui),
        (Some("btc".to_string()), None),
        "tab returns to the unit clicked last"
    );
}

// ---------------------------------------------------------------------------------------------
// The X-Large pane for a selected Price Source (DESKTOP-140, #716)
// ---------------------------------------------------------------------------------------------

#[gpui::test]
fn at_x_large_clicking_a_price_source_swaps_the_pane_to_its_groups(app: &mut TestAppContext) {
    let mut ui = on_units_xlarge(app);
    assert!(ui.is_drawn("units-pane-unit"));

    ui.click("price-source-row-0");
    assert_eq!(
        ui.settings_units().selected_source.as_deref(),
        Some("CoinGecko")
    );
    // `is_drawn` can't say a selector has gone (gpui keeps a frame's debug bounds), so the groups
    // are read back.
    assert_eq!(
        ui.settings_units().pane_groups,
        ["source", "values", "activity"]
    );
    assert!(ui.is_drawn("units-pane-values"));

    ui.click("unit-row-1");
    assert_eq!(
        ui.settings_units().pane_groups,
        ["unit", "source", "accounts", "activity"]
    );
}

#[gpui::test]
fn at_x_large_a_built_ins_pane_buttons_open_what_its_keys_do(app: &mut TestAppContext) {
    let mut ui = on_units_xlarge(app);
    ui.click("price-source-row-0");

    ui.click("units-pane-edit-source");
    assert_eq!(ui.dialog(), Some("EditBuiltInPriceSource"));
    assert_eq!(ui.dialog_value("name").as_deref(), Some("CoinGecko"));
    ui.press("escape");

    assert_eq!(
        ui.settings_units().pane_source_buttons,
        ["test-source", "edit-source"]
    );
    ui.click("units-pane-test-source");
    assert!(status(&mut ui).is_some(), "testing is not built yet");
}

#[gpui::test]
fn at_x_large_a_custom_sources_pane_buttons_open_its_edit_and_delete(app: &mut TestAppContext) {
    let mut ui = on_units_xlarge(app);
    add_custom_source(&mut ui, "broker");
    ui.click("price-source-row-0");
    assert_eq!(
        ui.settings_units().selected_source.as_deref(),
        Some("broker")
    );

    ui.click("units-pane-edit-source");
    assert_eq!(ui.dialog(), Some("EditPriceSource"));
    ui.press("escape");
    ui.click("units-pane-delete-source");
    assert_eq!(ui.dialog(), Some("DeletePriceSource"));
}

#[gpui::test]
fn at_x_large_manual_entrys_pane_has_no_buttons(app: &mut TestAppContext) {
    let mut ui = on_units_xlarge(app);
    ui.click("price-source-row-2");
    assert_eq!(
        ui.settings_units().selected_source.as_deref(),
        Some("Manual entry")
    );
    assert!(ui.is_drawn("units-pane-values"));
    assert!(ui.settings_units().pane_source_buttons.is_empty());
}

#[gpui::test]
fn add_unit_picks_a_source_and_quote_by_mouse_and_saves_the_link(app: &mut TestAppContext) {
    let mut ui = on_units(app);

    ui.click("settings-add-unit");
    ui.press("e t h");
    ui.click("add-unit-name");
    ui.press("e t h e r");
    ui.click("add-unit-type");
    ui.click("add-unit-type-option-1");
    assert_eq!(
        ui.dialog_value("sources").as_deref(),
        Some("CoinGecko|Manual entry")
    );
    ui.click("add-unit-source");
    ui.click("add-unit-source-option-0");
    assert_eq!(ui.dialog_value("source").as_deref(), Some("CoinGecko"));
    ui.click("add-unit-symbol");
    ui.press("e t h");
    ui.click("add-unit-quote");
    ui.click("add-unit-quote-option-1");
    ui.click("add-unit-confirm");
    assert_eq!(ui.dialog(), None);
    let page = ui.settings_units();
    let link = page.unit_links.last().cloned().unwrap();
    assert_eq!(link.0, "eth");
    assert_eq!(link.1, "CoinGecko");
    assert_eq!(link.2, "eth");
    assert_eq!(link.3, page.unit_codes[1], "the quote Unit picked by click");
    assert_eq!(
        page.unit_rows.last().map(|row| row.2.as_str()),
        Some("crypto")
    );
}

#[gpui::test]
fn choosing_manual_entry_by_mouse_hides_symbol_quote_and_test(app: &mut TestAppContext) {
    let mut ui = on_units(app);

    ui.click("settings-add-unit");
    assert!(!ui.is_drawn("add-unit-symbol"));
    ui.click("add-unit-type");
    ui.click("add-unit-type-option-1");
    ui.click("add-unit-source");
    ui.click("add-unit-source-option-0");
    assert!(ui.is_drawn("add-unit-symbol"));
    assert!(ui.is_drawn("add-unit-quote"));
    assert!(ui.is_drawn("add-unit-test"));
    ui.click("add-unit-source");
    ui.click("add-unit-source-option-1");
    assert_eq!(ui.dialog_value("manual").as_deref(), Some("true"));
    assert_eq!(
        ui.dialog_value("visible").as_deref(),
        Some("Code|Type|Name|Source")
    );
}

#[gpui::test]
fn the_edit_unit_source_list_follows_a_clicked_type_change(app: &mut TestAppContext) {
    let mut ui = on_units(app);

    row_action(&mut ui, "unit-row-1", "edit-unit");
    assert_eq!(ui.dialog_value("source").as_deref(), Some("CoinGecko"));
    ui.click("edit-unit-type");
    ui.click("edit-unit-type-option-2");
    assert_eq!(ui.dialog_value("source").as_deref(), Some("Manual entry"));
    assert_eq!(
        ui.dialog_value("visible").as_deref(),
        Some("Code|Type|Name|Source")
    );
}
