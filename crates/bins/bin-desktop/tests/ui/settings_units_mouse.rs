//! The Settings Units page driven by clicks: the index entry, the Add unit button, the row edit
//! and delete buttons, the Add, Edit and Delete unit dialogs' Type control, confirm and cancel
//! buttons, and the Price Sources buttons. State is read back from `Shell`.

use super::common;

use bin_desktop::{
    WindowSize,
    navigation::nav::{InputMode, Noun},
};
use common::Harness;
use gpui::TestAppContext;

/// Below X-Large the rows carry their own Edit and Delete buttons, so the older cases run at
/// Large; the X-Large cases below drive the detail pane.
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

#[gpui::test]
fn clicking_the_units_index_entry_swaps_to_the_page(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("g s");
    ui.click("settings-index-Accounts");
    assert_eq!(ui.settings().page, "Accounts");

    ui.click("settings-index-Units");
    assert_eq!(ui.settings().page, "Units");
    assert_eq!(ui.settings().dialog, None);
}

#[gpui::test]
fn the_add_unit_button_opens_and_cancel_closes_without_a_write(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let before = ui.settings_units().unit_rows.len();

    ui.click("settings-add-unit");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddUnit"));
    assert_eq!(mode(&mut ui), InputMode::Dialog);
    ui.click("add-unit-cancel");
    assert_eq!(ui.settings().dialog, None);
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
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddUnit"));
    assert_eq!(ui.settings_units().unit_rows.len(), before);
    ui.press("g o l d");
    ui.click("add-unit-confirm");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddUnit"));
    assert_eq!(ui.settings_units().unit_rows.len(), before);
}

#[gpui::test]
fn the_add_unit_type_control_and_confirm_button_add_the_unit(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let before = ui.settings_units().unit_rows.len();

    ui.click("settings-add-unit");
    ui.press("g o l d");
    ui.press("tab");
    ui.press("g o l d");
    ui.click("add-unit-type-4");
    assert_eq!(
        ui.settings_units().unit_dialog_kind.as_deref(),
        Some("Other")
    );
    ui.click("add-unit-confirm");
    let units_page = ui.settings_units();
    assert_eq!(ui.settings().dialog, None);
    assert_eq!(units_page.unit_rows.len(), before + 1);
    assert_eq!(
        units_page
            .unit_rows
            .last()
            .map(|row| (row.0.as_str(), row.2.as_str())),
        Some(("gold", "other"))
    );
}

#[gpui::test]
fn the_edit_unit_button_opens_a_dialog_prefilled_from_the_row(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let (code, name, kind) = ui.settings_units().unit_rows[0].clone();

    ui.click("unit-edit-0");
    let units_page = ui.settings_units();
    let page = ui.settings();
    assert_eq!(page.dialog.as_deref(), Some("EditUnit"));
    assert_eq!(page.dialog_name.as_deref(), Some(code.as_str()));
    assert_eq!(units_page.unit_dialog_name.as_deref(), Some(name.as_str()));
    ui.click("edit-unit-cancel");
    assert_eq!(ui.settings().dialog, None);
    assert_eq!(ui.settings_units().unit_rows[0], (code, name, kind));
}

#[gpui::test]
fn the_edit_unit_dialog_prefills_the_stored_type(app: &mut TestAppContext) {
    let mut ui = on_units(app);

    // vas is seeded as a Stock, which the old three-option selector could only show as Custom.
    ui.click("unit-edit-2");
    assert_eq!(
        ui.settings_units().unit_dialog_kind.as_deref(),
        Some("Stock")
    );
}

#[gpui::test]
fn the_edit_unit_dialog_saves_a_changed_type_by_mouse(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let (code, name, _) = ui.settings_units().unit_rows[0].clone();

    ui.click("unit-edit-0");
    ui.click("edit-unit-type-1");
    assert_eq!(
        ui.settings_units().unit_dialog_kind.as_deref(),
        Some("Crypto")
    );
    ui.click("edit-unit-confirm");
    let units_page = ui.settings_units();
    assert_eq!(ui.settings().dialog, None);
    assert_eq!(units_page.unit_rows[0], (code, name, "crypto".to_string()));
}

#[gpui::test]
fn the_delete_unit_button_opens_a_dialog_and_cancel_closes_without_a_write(
    app: &mut TestAppContext,
) {
    let mut ui = on_units(app);
    let before = ui.settings_units().unit_codes.clone();

    ui.click("unit-delete-0");
    assert_eq!(ui.settings().dialog.as_deref(), Some("DeleteUnit"));
    ui.click("delete-unit-cancel");
    assert_eq!(ui.settings().dialog, None);
    assert_eq!(ui.settings_units().unit_codes, before);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn the_delete_unit_confirm_button_refuses_until_the_code_is_typed_back(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let before = ui.settings_units().unit_codes.clone();
    let code = before[0].clone();

    ui.click("unit-delete-0");
    ui.click("delete-unit-confirm");
    assert_eq!(ui.settings().dialog.as_deref(), Some("DeleteUnit"));
    assert_eq!(ui.settings_units().unit_codes, before);

    ui.press(
        &code
            .chars()
            .next()
            .expect("seeded units have codes")
            .to_string(),
    );
    ui.click("delete-unit-confirm");
    assert_eq!(ui.settings().dialog.as_deref(), Some("DeleteUnit"));
    assert_eq!(ui.settings_units().unit_codes, before);
}

#[gpui::test]
fn the_delete_unit_confirm_button_removes_the_row_once_the_code_matches(app: &mut TestAppContext) {
    let mut ui = on_units(app);
    let before = ui.settings_units().unit_codes.clone();
    let code = before[0].clone();

    ui.click("unit-delete-0");
    for ch in code.chars() {
        ui.press(&ch.to_string());
    }
    ui.click("delete-unit-confirm");
    let after = ui.settings_units().unit_codes;
    assert_eq!(ui.settings().dialog, None);
    assert_eq!(after.len(), before.len() - 1);
    assert!(!after.contains(&code));
}

#[gpui::test]
fn the_price_source_buttons_flash_not_yet_built_messages(app: &mut TestAppContext) {
    let mut ui = on_units(app);

    ui.click("price-source-test-0");
    assert_eq!(
        status(&mut ui).as_deref(),
        Some("test price source \u{2014} not yet built")
    );
    ui.click("price-source-edit-0");
    assert_eq!(
        status(&mut ui).as_deref(),
        Some("edit price source \u{2014} not yet built")
    );
    ui.click("price-source-delete-1");
    assert_eq!(
        status(&mut ui).as_deref(),
        Some("delete price source \u{2014} not yet built")
    );
    ui.click("settings-add-price-source");
    assert_eq!(
        status(&mut ui).as_deref(),
        Some("add price source \u{2014} not yet built")
    );
    assert_eq!(ui.settings().dialog, None);
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
fn at_x_large_the_rows_drop_their_buttons_and_the_pane_docks(app: &mut TestAppContext) {
    let mut ui = on_units_xlarge(app);
    assert!(ui.is_drawn("units-detail-pane"));
    assert!(!ui.is_drawn("unit-edit-0"));
    assert!(!ui.is_drawn("unit-delete-0"));
    assert!(!ui.is_drawn("price-source-test-0"));
    assert!(ui.is_drawn("units-pane-unit"));
    assert!(ui.is_drawn("units-pane-source"));
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
    assert_eq!(ui.settings().dialog.as_deref(), Some("EditUnit"));
    ui.press("escape");

    ui.click("units-pane-delete-unit");
    assert_eq!(ui.settings().dialog.as_deref(), Some("DeleteUnit"));
    ui.click("delete-unit-cancel");
    assert_eq!(ui.settings().dialog, None);
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
    ui.click("unit-delete-1");
    for ch in "btc".chars() {
        ui.press(&ch.to_string());
    }
    ui.click("delete-unit-confirm");

    assert!(ui.settings_units().activity.is_empty());
    assert_eq!(ui.toasts().visible.len(), 1);
}
