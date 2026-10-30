//! The Settings Institutions page driven by clicks: the Institutions row and add buttons, and the
//! Add institution dialog's Products chips and buttons, and the Delete institution dialog's
//! buttons in its two states. State is read back from `Root`.

use super::common;
use bin_desktop::shell::router::SettingsInstitutionsSnapshot;

use bin_desktop::navigation::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;

fn on_page<'a>(app: &'a mut TestAppContext, page: &str) -> Harness<'a> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    ui.click(&format!("settings-index-{page}"));
    assert_eq!(ui.settings_frame().page, page);
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

    assert_eq!(
        ui.settings::<SettingsInstitutionsSnapshot>()
            .institutions
            .len(),
        7
    );
    assert!(ui.is_drawn("settings-institutions-row-6"));
    ui.click("institution-edit-0");
    assert_eq!(ui.dialog_value("name").as_deref(), Some("Personal"));
}

#[gpui::test]
fn the_edit_link_opens_the_edit_dialog_and_the_delete_link_the_delete_dialog(
    app: &mut TestAppContext,
) {
    let mut ui = on_page(app, "Institutions");
    let before = ui.settings::<SettingsInstitutionsSnapshot>().institutions;

    ui.click("institution-edit-2");
    assert_eq!(ui.dialog(), Some("EditInstitution"));
    assert_eq!(ui.dialog_value("name").as_deref(), Some("American Express"));
    assert_eq!(ui.dialog_value("types").as_deref(), Some("CreditCard"));
    ui.click("add-institution-cancel");
    assert_eq!(ui.dialog(), None);

    ui.click("institution-delete-2");
    assert_eq!(ui.dialog(), Some("DeleteInstitution"));
    assert_eq!(
        ui.settings::<SettingsInstitutionsSnapshot>()
            .selected_institution
            .as_deref(),
        Some("American Express")
    );
    ui.click("delete-institution-close");
    assert_eq!(ui.dialog(), None);
    assert_eq!(
        ui.settings::<SettingsInstitutionsSnapshot>().institutions,
        before
    );
}

#[gpui::test]
fn a_right_click_opens_the_rows_menu_and_edit_opens_the_dialog_on_that_row(
    app: &mut TestAppContext,
) {
    let mut ui = on_page(app, "Institutions");

    ui.right_click("settings-institutions-row-3");
    assert!(ui.is_drawn("settings-institutions-menu"));
    assert_eq!(
        ui.settings::<SettingsInstitutionsSnapshot>().menu,
        Some(vec!["edit-institution", "delete-institution"])
    );
    assert_eq!(
        ui.settings::<SettingsInstitutionsSnapshot>()
            .selected_institution
            .as_deref(),
        Some("Vanguard Investments")
    );
    assert!(ui.settings_frame().page_focused);
    ui.click("settings-institutions-menu-edit-institution");

    assert_eq!(ui.settings::<SettingsInstitutionsSnapshot>().menu, None);
    assert_eq!(ui.dialog(), Some("EditInstitution"));
    assert_eq!(
        ui.dialog_value("name").as_deref(),
        Some("Vanguard Investments")
    );
}

#[gpui::test]
fn the_menus_delete_opens_the_delete_dialog_and_a_press_outside_closes_the_menu(
    app: &mut TestAppContext,
) {
    let mut ui = on_page(app, "Institutions");

    row_action(&mut ui, "settings-institutions-row-0", "delete-institution");
    assert_eq!(ui.dialog(), Some("DeleteInstitution"));
    assert_eq!(ui.dialog_value("name"), None);
    assert_eq!(ui.dialog_value("refused").as_deref(), Some("true"));
    ui.click("delete-institution-close");
    assert_eq!(ui.dialog(), None);

    assert_eq!(ui.settings::<SettingsInstitutionsSnapshot>().menu, None);

    ui.right_click("settings-institutions-row-1");
    assert!(ui.settings::<SettingsInstitutionsSnapshot>().menu.is_some());
    ui.click("settings-institutions-row-0");
    assert_eq!(ui.settings::<SettingsInstitutionsSnapshot>().menu, None);
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn a_refusal_lists_the_accounts_and_offers_only_close(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");

    ui.click("institution-delete-1");

    assert_eq!(
        ui.dialog_value("notice").as_deref(),
        Some("Used by 2 accounts")
    );
    assert!(ui.is_drawn("delete-institution-account-0"));
    assert!(ui.is_drawn("delete-institution-account-1"));
    assert!(!ui.is_drawn("delete-institution-account-2"));
    assert!(ui.is_drawn("delete-institution-close"));
    assert!(
        !ui.is_drawn("delete-institution-confirm"),
        "no destructive button"
    );
    assert!(!ui.is_drawn("delete-institution-cancel"));
    ui.click("delete-institution-close");
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn the_delete_button_enables_on_the_name_typed_back_and_removes_the_row(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");

    ui.click("institution-delete-6");
    assert_eq!(ui.dialog_value("refused").as_deref(), Some("false"));
    ui.click("delete-institution-confirm");
    assert_eq!(
        ui.dialog(),
        Some("DeleteInstitution"),
        "disabled until typed back"
    );
    ui.press("S u p e r a n n u a t i o n space F u n d");
    assert_eq!(ui.dialog_value("can_confirm").as_deref(), Some("true"));
    ui.click("delete-institution-confirm");

    assert_eq!(ui.dialog(), None);
    let page = ui.settings::<SettingsInstitutionsSnapshot>();
    assert_eq!(page.institutions.len(), 6);
    assert_eq!(
        page.selected_institution.as_deref(),
        Some("Cryptocurrency Exchange")
    );
}

#[gpui::test]
fn cancel_closes_the_delete_dialog_with_nothing_removed(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");
    let before = ui.settings::<SettingsInstitutionsSnapshot>().institutions;

    row_action(&mut ui, "settings-institutions-row-6", "delete-institution");
    ui.press("S u p e r a n n u a t i o n space F u n d");
    ui.click("delete-institution-cancel");

    assert_eq!(ui.dialog(), None);
    assert_eq!(
        ui.settings::<SettingsInstitutionsSnapshot>().institutions,
        before
    );
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
        ui.settings::<SettingsInstitutionsSnapshot>().institutions[2],
        ("Amex".to_string(), "credit card \u{b7} cash".to_string())
    );
}

#[gpui::test]
fn the_add_institution_dialog_seeds_savings_and_a_usual_unit_of_none(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");

    ui.click("settings-add-institution");
    assert_eq!(ui.dialog(), Some("AddInstitution"));
    assert_eq!(ui.dialog_value("types").as_deref(), Some("Savings"));
    assert_eq!(ui.dialog_value("usual_unit").as_deref(), Some("None"));
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
    let before = ui
        .settings::<SettingsInstitutionsSnapshot>()
        .institutions
        .len();

    ui.click("settings-add-institution");
    ui.click("add-institution-confirm");
    assert_eq!(ui.dialog(), Some("AddInstitution"));
    ui.press("n e w");
    ui.click("add-institution-chip-offset");
    ui.click("add-institution-confirm");
    let institutions_page = ui.settings::<SettingsInstitutionsSnapshot>();
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
    let before = ui
        .settings::<SettingsInstitutionsSnapshot>()
        .institutions
        .len();

    ui.click("settings-add-institution");
    ui.press("n e w");
    ui.click("add-institution-chip-savings");
    ui.click("add-institution-confirm");
    assert_eq!(ui.dialog(), Some("AddInstitution"));
    assert_eq!(
        ui.settings::<SettingsInstitutionsSnapshot>()
            .institutions
            .len(),
        before
    );
}

#[gpui::test]
fn a_row_click_selects_its_institution_and_a_row_button_leaves_the_selection_on_that_row(
    app: &mut TestAppContext,
) {
    let mut ui = on_page(app, "Institutions");
    let names: Vec<String> = ui
        .settings::<SettingsInstitutionsSnapshot>()
        .institutions
        .into_iter()
        .map(|row| row.0)
        .collect();

    ui.click("settings-institutions-row-2");
    assert_eq!(
        ui.settings::<SettingsInstitutionsSnapshot>()
            .selected_institution
            .as_ref(),
        names.get(2)
    );
    ui.click("institution-delete-4");
    assert_eq!(
        ui.settings::<SettingsInstitutionsSnapshot>()
            .selected_institution
            .as_ref(),
        names.get(4)
    );
    assert_eq!(ui.dialog(), Some("DeleteInstitution"));
    ui.click("delete-institution-close");
    assert_eq!(ui.dialog(), None);
}

#[gpui::test]
fn escape_closes_the_open_menu_before_it_leaves_the_page(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");

    ui.right_click("settings-institutions-row-1");
    assert!(ui.settings::<SettingsInstitutionsSnapshot>().menu.is_some());
    ui.press("escape");

    assert_eq!(ui.settings::<SettingsInstitutionsSnapshot>().menu, None);
    assert_eq!(ui.settings_frame().page, "Institutions");
    assert!(ui.settings_frame().page_focused);
}

#[gpui::test]
fn the_usual_unit_select_opens_closes_and_picks_a_unit_by_click(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");

    ui.click("settings-add-institution");
    ui.click("institution-usual-unit");
    assert_eq!(ui.dialog_value("focused").as_deref(), Some("UsualUnit"));
    ui.click("institution-usual-unit-option-2");
    assert_eq!(ui.dialog_value("usual_unit").as_deref(), Some("btc"));

    ui.click("institution-usual-unit");
    ui.click("institution-usual-unit-option-0");
    assert_eq!(ui.dialog_value("usual_unit").as_deref(), Some("None"));
}

#[gpui::test]
fn a_usual_unit_is_saved_reopens_intact_and_clears_back_to_none(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");
    let last = ui
        .settings::<SettingsInstitutionsSnapshot>()
        .institutions
        .len();

    ui.click("settings-add-institution");
    ui.press("b r o k e r");
    ui.click("institution-usual-unit");
    ui.click("institution-usual-unit-option-1");
    ui.click("add-institution-confirm");
    assert_eq!(ui.dialog(), None);

    let row = format!("settings-institutions-row-{last}");
    row_action(&mut ui, &row, "edit-institution");
    assert_eq!(ui.dialog_value("usual_unit").as_deref(), Some("aud"));
    ui.click("institution-usual-unit");
    ui.click("institution-usual-unit-option-0");
    ui.click("add-institution-confirm");

    row_action(&mut ui, &row, "edit-institution");
    assert_eq!(ui.dialog_value("usual_unit").as_deref(), Some("None"));
}

#[gpui::test]
fn the_seeded_exchange_opens_its_edit_dialog_on_its_usual_unit(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");

    row_action(&mut ui, "settings-institutions-row-5", "edit-institution");

    assert_eq!(
        ui.dialog_value("name").as_deref(),
        Some("Cryptocurrency Exchange")
    );
    assert_eq!(ui.dialog_value("usual_unit").as_deref(), Some("btc"));
}

#[gpui::test]
fn a_click_on_the_name_closes_the_usual_unit_list_and_takes_focus_back(app: &mut TestAppContext) {
    let mut ui = on_page(app, "Institutions");

    ui.click("settings-add-institution");
    ui.click("institution-usual-unit");
    ui.click("add-institution-name");

    assert_eq!(ui.dialog_value("focused").as_deref(), Some("Name"));
    ui.press("n e w");
    assert_eq!(ui.dialog_value("name").as_deref(), Some("new"));
}
