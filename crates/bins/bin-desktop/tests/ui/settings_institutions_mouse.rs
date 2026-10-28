//! The Settings Institutions page driven by clicks: the Institutions row and add buttons, and the
//! Add institution dialog's Products chips and buttons. State is read back from `Root`.

use super::common;

use bin_desktop::navigation::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;

fn on_page<'a>(app: &'a mut TestAppContext, page: &str) -> Harness<'a> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    ui.click(&format!("settings-index-{page}"));
    assert_eq!(ui.settings().page, page);
    ui
}

fn status(ui: &mut Harness<'_>) -> Option<String> {
    ui.read(|shell| shell.status_message())
}

/// Right-clicks the row and picks an entry of its context menu: the rows carry no buttons.
fn row_action(ui: &mut Harness<'_>, row: &str, action: &str) {
    ui.right_click(row);
    ui.click(&format!("settings-institutions-menu-{action}"));
}

#[gpui::test]
fn the_page_draws_seven_rows_the_personal_institution_first(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");

    assert_eq!(ui.settings_institutions().institutions.len(), 7);
    assert!(ui.is_drawn("settings-institutions-row-6"));
    ui.click("institution-edit-0");
    assert_eq!(ui.dialog_value("name").as_deref(), Some("Personal"));
}

#[gpui::test]
fn the_edit_link_opens_the_edit_dialog_and_the_delete_link_says_it_is_not_built(
    app: &mut TestAppContext,
) {
    let mut ui = on_page(app, "Institutions");
    let before = ui.settings_institutions().institutions;

    ui.click("institution-edit-2");
    assert_eq!(ui.dialog(), Some("EditInstitution"));
    assert_eq!(ui.dialog_value("name").as_deref(), Some("American Express"));
    assert_eq!(ui.dialog_value("types").as_deref(), Some("CreditCard"));
    ui.click("add-institution-cancel");
    assert_eq!(ui.dialog(), None);

    ui.click("institution-delete-2");
    assert_eq!(
        status(&mut ui).as_deref(),
        Some("delete institution \u{2014} not yet built")
    );
    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.settings_institutions().institutions, before);
}

#[gpui::test]
fn a_right_click_opens_the_rows_menu_and_edit_opens_the_dialog_on_that_row(
    app: &mut TestAppContext,
) {
    let mut ui = on_page(app, "Institutions");

    ui.right_click("settings-institutions-row-3");
    assert!(ui.is_drawn("settings-institutions-menu"));
    assert_eq!(
        ui.settings_institutions().menu,
        Some(vec!["edit-institution", "delete-institution"])
    );
    assert_eq!(
        ui.settings_institutions().selected_institution.as_deref(),
        Some("Vanguard Investments")
    );
    assert!(ui.settings().page_focused);
    ui.click("settings-institutions-menu-edit-institution");

    assert_eq!(ui.settings_institutions().menu, None);
    assert_eq!(ui.dialog(), Some("EditInstitution"));
    assert_eq!(
        ui.dialog_value("name").as_deref(),
        Some("Vanguard Investments")
    );
}

#[gpui::test]
fn the_menus_delete_says_it_is_not_built_and_a_press_outside_closes_the_menu(
    app: &mut TestAppContext,
) {
    let mut ui = on_page(app, "Institutions");

    row_action(&mut ui, "settings-institutions-row-0", "delete-institution");
    assert_eq!(
        status(&mut ui).as_deref(),
        Some("delete institution \u{2014} not yet built")
    );
    assert_eq!(ui.dialog(), None);

    assert_eq!(ui.settings_institutions().menu, None);

    ui.right_click("settings-institutions-row-1");
    assert!(ui.settings_institutions().menu.is_some());
    ui.click("settings-institutions-row-0");
    assert_eq!(ui.settings_institutions().menu, None);
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn the_edit_dialogs_buttons_save_a_rename_and_a_refused_one_stays_open(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");

    row_action(&mut ui, "settings-institutions-row-2", "edit-institution");
    for _ in 0.."American Express".len() {
        ui.press("backspace");
    }
    ui.press("a n z space b a n k i n g space g r o u p");
    ui.click("add-institution-confirm");
    assert_eq!(ui.dialog(), Some("EditInstitution"));

    for _ in 0.."anz banking group".len() {
        ui.press("backspace");
    }
    ui.press("A m e x");
    ui.click("add-institution-chip-cash");
    ui.click("add-institution-confirm");

    assert_eq!(ui.dialog(), None);
    assert_eq!(
        ui.settings_institutions().institutions[2],
        ("Amex".to_string(), "credit card \u{b7} cash".to_string())
    );
}

#[gpui::test]
fn the_add_institution_dialog_seeds_savings_and_asks_for_no_default_unit(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");

    ui.click("settings-add-institution");
    assert_eq!(ui.dialog(), Some("AddInstitution"));
    assert_eq!(ui.dialog_value("types").as_deref(), Some("Savings"));
    assert_eq!(ui.dialog_value("unit"), None);
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Dialog);
    ui.click("add-institution-cancel");
    assert_eq!(ui.dialog(), None);
    assert_eq!(ui.read(|shell| shell.nav().mode()), InputMode::Normal);
}

#[gpui::test]
fn products_chips_toggle_the_new_ones_among_them(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");

    ui.click("settings-add-institution");
    ui.click("add-institution-chip-offset");
    assert_eq!(ui.dialog_value("types").as_deref(), Some("Savings,Offset"));
    ui.click("add-institution-chip-savings");
    assert_eq!(ui.dialog_value("types").as_deref(), Some("Offset"));
    ui.click("add-institution-chip-cash");
    ui.click("add-institution-chip-crypto");
    assert_eq!(
        ui.dialog_value("types").as_deref(),
        Some("Offset,Crypto,Cash")
    );
}

#[gpui::test]
fn the_add_institution_button_needs_a_name_and_a_product_then_adds_the_row(
    app: &mut TestAppContext,
) {
    let mut ui = on_page(app, "Institutions");
    let before = ui.settings_institutions().institutions.len();

    ui.click("settings-add-institution");
    ui.click("add-institution-confirm");
    assert_eq!(ui.dialog(), Some("AddInstitution"));
    ui.press("n e w");
    ui.click("add-institution-chip-offset");
    ui.click("add-institution-confirm");
    let institutions_page = ui.settings_institutions();
    assert_eq!(ui.dialog(), None);
    assert_eq!(institutions_page.institutions.len(), before + 1);
    assert_eq!(
        institutions_page.institutions.last(),
        Some(&("new".to_string(), "savings \u{b7} offset".to_string()))
    );
}

#[gpui::test]
fn an_institution_with_no_product_cannot_be_added(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");
    let before = ui.settings_institutions().institutions.len();

    ui.click("settings-add-institution");
    ui.press("n e w");
    ui.click("add-institution-chip-savings");
    ui.click("add-institution-confirm");
    assert_eq!(ui.dialog(), Some("AddInstitution"));
    assert_eq!(ui.settings_institutions().institutions.len(), before);
}

#[gpui::test]
fn a_row_click_selects_its_institution_and_a_row_button_leaves_the_selection_on_that_row(
    app: &mut TestAppContext,
) {
    let mut ui = on_page(app, "Institutions");
    let names: Vec<String> = ui
        .settings_institutions()
        .institutions
        .into_iter()
        .map(|row| row.0)
        .collect();

    ui.click("settings-institutions-row-2");
    assert_eq!(
        ui.settings_institutions().selected_institution.as_ref(),
        names.get(2)
    );
    ui.click("institution-delete-4");
    assert_eq!(
        ui.settings_institutions().selected_institution.as_ref(),
        names.get(4)
    );
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn escape_closes_the_open_menu_before_it_leaves_the_page(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");

    ui.right_click("settings-institutions-row-1");
    assert!(ui.settings_institutions().menu.is_some());
    ui.press("escape");

    assert_eq!(ui.settings_institutions().menu, None);
    assert_eq!(ui.settings().page, "Institutions");
    assert!(ui.settings().page_focused);
}
