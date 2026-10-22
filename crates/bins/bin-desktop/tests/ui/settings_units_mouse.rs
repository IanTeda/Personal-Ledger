//! The Settings Units page driven by clicks, on in-memory Units ports. Wiring and drawing only: a
//! click on a row, a menu entry, a **+**, a pane button or a Dialog control reaches the `lib-units`
//! Presenter as an event, and the page or Dialog draws the view model it builds. The rules are the
//! Presenter tests' in `lib-units`.

#![expect(
    clippy::expect_used,
    reason = "test code: a row missing from the page should fail the test loudly"
)]

use super::common;

use bin_desktop::{WindowSize, navigation::nav::Noun};
use common::Harness;
use gpui::TestAppContext;

fn on_units_at(app: &mut TestAppContext, size: WindowSize) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.resize_to(size);
    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    ui.click("settings-index-Units");
    assert_eq!(ui.settings().page, "Units");
    ui
}

fn on_units(app: &mut TestAppContext) -> Harness<'_> {
    on_units_at(app, WindowSize::Large)
}

/// The selected row: a Unit's code or a Price Source's name.
fn selected(ui: &mut Harness<'_>) -> String {
    let page = ui.settings_units();
    page.units
        .iter()
        .find(|unit| unit.selected)
        .map(|unit| unit.code.clone())
        .or_else(|| {
            page.price_sources
                .iter()
                .find(|source| source.selected)
                .map(|source| source.name.clone())
        })
        .unwrap_or_default()
}

/// The open row menu's entries as their slugs.
fn menu(ui: &mut Harness<'_>) -> Option<Vec<&'static str>> {
    ui.settings_units().menu.map(|menu| {
        menu.entries
            .into_iter()
            .map(|entry| entry.action.slug())
            .collect()
    })
}

fn source_row(ui: &mut Harness<'_>, name: &str) -> String {
    let index = ui
        .settings_units()
        .price_sources
        .iter()
        .position(|source| source.name == name)
        .expect("a source by that name is listed");
    format!("price-source-row-{index}")
}

#[gpui::test]
fn clicking_the_units_index_entry_swaps_to_the_page(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("g s");
    ui.click("settings-index-Accounts");
    assert_eq!(ui.settings().page, "Accounts");

    ui.click("settings-index-Units");
    assert_eq!(ui.settings().page, "Units");
    assert!(ui.is_drawn("settings-units-table"));
    assert!(ui.is_drawn("settings-price-sources-table"));
}

#[gpui::test]
fn clicking_a_row_selects_it_and_focuses_the_page(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    assert!(!ui.settings().page_focused);

    ui.click("unit-row-2");
    assert_eq!(selected(&mut ui), "vas");
    assert!(ui.settings().page_focused);
    let row = source_row(&mut ui, "Frankfurter");
    ui.click(&row);
    assert_eq!(selected(&mut ui), "Frankfurter");
}

#[gpui::test]
fn double_clicking_a_row_opens_its_edit_dialog(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.double_click("unit-row-1");
    assert_eq!(ui.dialog(), Some("EditUnit"));
    assert_eq!(ui.dialog_value("code").as_deref(), Some("btc"));
    ui.click("edit-unit-cancel");

    let row = source_row(&mut ui, "CoinGecko");
    ui.double_click(&row);
    assert_eq!(ui.dialog(), Some("EditBuiltInPriceSource"));
}

#[gpui::test]
fn a_right_click_opens_the_rows_menu_and_an_entry_runs_its_action(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.right_click("unit-row-1");
    assert_eq!(selected(&mut ui), "btc");
    assert_eq!(menu(&mut ui), Some(vec!["edit-unit", "delete-unit"]));
    assert!(ui.is_drawn("settings-units-menu"));

    ui.click("settings-units-menu-delete-unit");
    assert_eq!(menu(&mut ui), None);
    assert_eq!(ui.dialog(), Some("DeleteUnit"));
}

#[gpui::test]
fn a_press_outside_the_menu_closes_it(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let row = source_row(&mut ui, "CoinGecko");
    ui.right_click(&row);
    assert_eq!(menu(&mut ui), Some(vec!["test-source", "edit-source"]));

    ui.click("unit-row-0");
    assert_eq!(menu(&mut ui), None);
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn the_plus_buttons_open_their_tables_add_dialogs(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.click("settings-add-price-source");
    assert_eq!(ui.dialog(), Some("AddPriceSource"));
    ui.click("add-price-source-cancel");
    ui.click("settings-add-unit");
    assert_eq!(ui.dialog(), Some("AddUnit"));
    ui.click("add-unit-cancel");
    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.unit_codes(), ["aud", "btc", "vas"]);
}

#[gpui::test]
fn the_add_unit_controls_and_confirm_button_add_the_unit(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.click("settings-add-unit");
    ui.click("add-unit-confirm");
    assert_eq!(ui.dialog(), Some("AddUnit"), "disabled until Code and Name");

    ui.press("g o l d");
    ui.click("add-unit-type");
    assert_eq!(ui.dialog_value("select_open").as_deref(), Some("true"));
    ui.click("add-unit-type-option-4");
    assert_eq!(ui.dialog_value("kind").as_deref(), Some("Commodity"));
    ui.click("add-unit-name");
    ui.press("G o l d");
    ui.click("add-unit-confirm");

    assert_eq!(ui.dialog(), None);
    assert_eq!(
        ui.unit_rows().last(),
        Some(&(
            "gold".to_string(),
            "Gold".to_string(),
            "Commodity".to_string()
        ))
    );
}

#[gpui::test]
fn choosing_a_source_by_click_shows_the_symbol_and_quote_fields(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.click("settings-add-unit");
    assert!(!ui.is_drawn("add-unit-symbol"));
    assert_eq!(
        ui.dialog_value("visible").as_deref(),
        Some("Code|Type|Name|Source")
    );

    ui.click("add-unit-source");
    let sources = ui.dialog_value("sources").unwrap_or_default();
    let custom_or_built_in = sources
        .split('|')
        .position(|source| source != "Manual entry")
        .expect("a fetching source on offer");
    ui.click(&format!("add-unit-source-option-{custom_or_built_in}"));
    assert_eq!(
        ui.dialog_value("visible").as_deref(),
        Some("Code|Type|Name|Source|Symbol|Quote")
    );
    assert!(ui.is_drawn("add-unit-symbol"));
}

#[gpui::test]
fn the_delete_confirm_button_shows_the_refusal_then_deactivates(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.right_click("unit-row-1");
    ui.click("settings-units-menu-delete-unit");
    ui.press("b t c");
    ui.click("delete-unit-confirm");
    assert_eq!(ui.dialog_value("in_use").as_deref(), Some("true"));

    ui.click("delete-unit-confirm");
    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.inactive_unit_codes(), ["btc"]);

    ui.right_click("unit-row-1");
    assert_eq!(
        menu(&mut ui),
        Some(vec!["edit-unit", "reactivate-unit", "delete-unit"])
    );
    ui.click("settings-units-menu-reactivate-unit");
    assert!(ui.inactive_unit_codes().is_empty());
}

#[gpui::test]
fn the_new_base_select_moves_the_base_by_click(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.drop_unit_users();
    ui.unquote_units();
    ui.right_click("unit-row-0");
    ui.click("settings-units-menu-delete-unit");
    ui.click("delete-unit-new-base");
    assert_eq!(ui.dialog_value("select_open").as_deref(), Some("true"));
    ui.click("delete-unit-new-base-option-1");
    assert_eq!(
        ui.dialog_value("new_base").as_deref(),
        Some("vas — Vanguard Aus Shares")
    );
    ui.press("a u d");
    ui.click("delete-unit-confirm");

    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.base_unit_code().as_deref(), Some("vas"));
}

#[gpui::test]
fn a_built_ins_updates_control_and_save_button_rename_it(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let row = source_row(&mut ui, "Frankfurter");
    ui.double_click(&row);
    assert_eq!(ui.dialog(), Some("EditBuiltInPriceSource"));
    assert!(ui.is_drawn("edit-built-in-source-facts"));

    ui.click("edit-built-in-source-updates-2");
    assert_eq!(ui.dialog_value("updates").as_deref(), Some("Daily"));
    ui.press("x");
    ui.click("edit-built-in-source-confirm");

    assert_eq!(ui.dialog(), None);
    let names: Vec<String> = ui
        .settings_units()
        .price_sources
        .into_iter()
        .map(|source| source.name)
        .collect();
    assert!(names.contains(&"Frankfurterx".to_string()));
}

#[gpui::test]
fn manual_entry_opens_no_menu_and_no_dialog(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let row = source_row(&mut ui, "Manual entry");
    ui.right_click(&row);
    assert_eq!(menu(&mut ui), None);
    ui.double_click(&row);
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn at_x_large_the_pane_draws_its_groups_and_its_buttons_open_the_dialogs(app: &mut TestAppContext) {
    let mut ui = on_units_at(app, WindowSize::XLarge);
    for group in ["unit", "source", "accounts", "activity"] {
        assert!(ui.is_drawn(&format!("units-pane-{group}")), "{group}");
    }

    ui.click("units-pane-add-source");
    assert_eq!(ui.dialog(), Some("AddPriceSource"));
    ui.click("add-price-source-cancel");
    ui.click("unit-row-1");
    ui.click("units-pane-edit-source");
    assert_eq!(ui.dialog(), Some("EditBuiltInPriceSource"));
    ui.click("edit-built-in-source-cancel");
    ui.click("units-pane-edit-unit");
    assert_eq!(ui.dialog(), Some("EditUnit"));
    assert_eq!(ui.dialog_value("code").as_deref(), Some("btc"));
}

#[gpui::test]
fn at_x_large_the_pane_draws_a_units_accounts(app: &mut TestAppContext) {
    let mut ui = on_units_at(app, WindowSize::XLarge);
    ui.click("unit-row-1");
    let page = ui.settings_units();
    let accounts = page
        .pane
        .expect("docked")
        .groups
        .into_iter()
        .find_map(|group| match group {
            lib_units::presenter::PaneGroup::Accounts(accounts) => Some(accounts),
            _ => None,
        })
        .expect("an ACCOUNTS group");
    assert!(!accounts.rows.is_empty(), "btc's stub Accounts");
    assert!(accounts.rows.iter().all(|row| {
        row.value
            .as_deref()
            .is_some_and(|value| value.ends_with(" aud"))
    }));
}

#[gpui::test]
fn the_base_glyph_follows_the_unit_picked_on_general(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    assert_eq!(ui.base_unit_code().as_deref(), Some("aud"));
    ui.click("settings-index-General");
    ui.press("l j");
    ui.click("settings-index-Units");
    assert_ne!(ui.base_unit_code().as_deref(), Some("aud"));
}

#[gpui::test]
fn the_menus_test_entry_puts_not_built_on_the_status_line(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let row = source_row(&mut ui, "CoinGecko");
    ui.right_click(&row);
    ui.click("settings-units-menu-test-source");
    assert_eq!(
        ui.read(|shell| shell.status_message().map(str::to_string))
            .as_deref(),
        Some("test price source \u{2014} not yet built")
    );
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn deleting_a_unit_other_units_quote_shows_the_refusal_and_keeps_it(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    ui.drop_unit_users();
    ui.right_click("unit-row-0");
    ui.click("settings-units-menu-delete-unit");
    ui.press("a u d");
    ui.click("delete-unit-confirm");

    assert_eq!(ui.dialog(), Some("DeleteUnit"));
    assert!(
        ui.dialog_value("refusal")
            .is_some_and(|text| text.starts_with("aud is the quote unit of 2 other units")),
        "the refusal says how many units quote it"
    );
    assert_eq!(ui.unit_codes(), ["aud", "btc", "vas"]);
}
