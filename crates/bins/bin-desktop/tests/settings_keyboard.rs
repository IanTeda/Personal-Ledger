//! The Settings pages driven by keys: the index rail's paged navigation, focus stepping into and
//! out of a page, the Accounts, Categories and Payees lists with their add, edit and delete
//! dialogs, and the About page. State is read back from `Shell`.

mod common;

use bin_desktop::nav::{InputMode, Noun};
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
    assert_eq!(ui.settings().page, page);
    ui.press("l");
    assert!(ui.settings().page_focused);
    ui
}

#[gpui::test]
fn opens_on_general_with_focus_on_the_index(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    let page = ui.settings();
    assert_eq!(page.page, "General");
    assert!(!page.page_focused);
    assert_eq!(page.dialog, None);
}

#[gpui::test]
fn j_and_k_swap_the_page_live(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    ui.press("j");
    assert_eq!(ui.settings().page, "Display");
    ui.press("j j");
    assert_eq!(ui.settings().page, "Institutions");
    ui.press("k");
    assert_eq!(ui.settings().page, "Units");
}

#[gpui::test]
fn l_steps_into_a_page_and_h_or_escape_steps_back_out(app: &mut TestAppContext) {
    let mut ui = in_page(app, 4, "Accounts");

    ui.press("h");
    assert!(!ui.settings().page_focused);
    ui.press("enter");
    assert!(ui.settings().page_focused);
    ui.press("escape");
    assert!(!ui.settings().page_focused);
}

#[gpui::test]
fn the_about_page_has_nothing_to_focus(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    ui.press("shift-g");
    assert_eq!(ui.settings().page, "About");
    ui.press("l");
    assert!(!ui.settings().page_focused);
}

#[gpui::test]
fn the_page_on_show_survives_leaving_and_returning(app: &mut TestAppContext) {
    let mut ui = on_settings(app);

    ui.press("j j j j");
    assert_eq!(ui.settings().page, "Accounts");
    ui.press("g d");
    assert_eq!(ui.noun(), Noun::Dashboard);
    ui.press("g s");
    let page = ui.settings();
    assert_eq!(page.page, "Accounts");
    assert!(!page.page_focused);
}

#[gpui::test]
fn accounts_rows_move_with_j_k_and_the_ends(app: &mut TestAppContext) {
    let mut ui = in_page(app, 4, "Accounts");

    let first = ui.settings().selected_account;
    assert_eq!(
        first.as_deref(),
        ui.settings().account_names.first().map(String::as_str)
    );
    ui.press("j");
    assert_eq!(
        ui.settings().selected_account,
        Some(ui.settings().account_names[1].clone())
    );
    ui.press("shift-g");
    let page = ui.settings();
    assert_eq!(page.selected_account.as_ref(), page.account_names.last());
    ui.press("g g");
    assert_eq!(ui.settings().selected_account, first);
}

#[gpui::test]
fn n_opens_add_account_and_escape_cancels_it(app: &mut TestAppContext) {
    let mut ui = in_page(app, 4, "Accounts");

    ui.press("n");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddAccount"));
    assert_eq!(mode(&mut ui), InputMode::Dialog);
    ui.press("escape");
    assert_eq!(ui.settings().dialog, None);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn typing_a_name_and_pressing_enter_adds_the_account(app: &mut TestAppContext) {
    let mut ui = in_page(app, 4, "Accounts");
    let before = ui.settings().account_names.len();

    ui.press("n");
    ui.press("r a i n y");
    assert_eq!(ui.settings().dialog_name.as_deref(), Some("rainy"));
    ui.press("enter");
    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert_eq!(page.account_names.len(), before + 1);
    assert!(page.account_names.iter().any(|name| name == "rainy"));
}

#[gpui::test]
fn e_edits_the_selected_account_and_enter_saves_the_new_name(app: &mut TestAppContext) {
    let mut ui = in_page(app, 4, "Accounts");
    let name = ui.settings().selected_account.expect("seeded accounts");

    ui.press("e");
    let page = ui.settings();
    assert_eq!(page.dialog.as_deref(), Some("EditAccount"));
    assert_eq!(page.dialog_name.as_deref(), Some(name.as_str()));
    ui.press("x");
    ui.press("enter");
    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert!(page.account_names.contains(&format!("{name}x")));
}

#[gpui::test]
fn d_deletes_an_account_only_once_its_name_is_typed_back(app: &mut TestAppContext) {
    let mut ui = in_page(app, 4, "Accounts");
    let before = ui.settings().account_names.len();
    let name = ui.settings().selected_account.expect("seeded accounts");

    ui.press("d");
    assert_eq!(ui.settings().dialog.as_deref(), Some("DeleteAccount"));
    ui.press("enter");
    assert_eq!(ui.settings().dialog.as_deref(), Some("DeleteAccount"));
    assert_eq!(ui.settings().account_names.len(), before);
    for ch in name.chars() {
        ui.press(&if ch == ' ' {
            "space".to_string()
        } else {
            ch.to_string()
        });
    }
    assert_eq!(ui.settings().dialog_confirm.as_deref(), Some(name.as_str()));
    ui.press("enter");
    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert_eq!(page.account_names.len(), before - 1);
    assert!(!page.account_names.contains(&name));
}

#[gpui::test]
fn payees_rows_move_and_the_dialogs_open(app: &mut TestAppContext) {
    let mut ui = in_page(app, 7, "Payees");

    let page = ui.settings();
    assert_eq!(page.selected_payee.as_ref(), page.payee_names.first());
    ui.press("j");
    let page = ui.settings();
    assert_eq!(page.selected_payee.as_ref(), page.payee_names.get(1));
    ui.press("e");
    assert_eq!(ui.settings().dialog.as_deref(), Some("EditPayee"));
    ui.press("escape");
    ui.press("d");
    assert_eq!(ui.settings().dialog.as_deref(), Some("DeletePayee"));
    ui.press("escape");
    ui.press("n");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddPayee"));
    ui.press("escape");
    assert_eq!(ui.settings().dialog, None);
}

#[gpui::test]
fn a_new_payee_is_added_by_typing_its_name(app: &mut TestAppContext) {
    let mut ui = in_page(app, 7, "Payees");
    let before = ui.settings().payee_names.len();

    ui.press("n");
    ui.press("z o o");
    assert_eq!(ui.settings().dialog_name.as_deref(), Some("zoo"));
    ui.press("enter");
    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert_eq!(page.payee_names.len(), before + 1);
    assert!(page.payee_names.iter().any(|name| name == "zoo"));
}

#[gpui::test]
fn categories_rows_move_and_the_dialogs_open(app: &mut TestAppContext) {
    let mut ui = in_page(app, 5, "Categories");

    assert_eq!(ui.settings().selected_category, None);
    ui.press("j");
    assert!(ui.settings().selected_category.is_some());
    ui.press("e");
    assert_eq!(ui.settings().dialog.as_deref(), Some("EditCategory"));
    ui.press("escape");
    assert_eq!(ui.settings().dialog, None);
    ui.press("shift-n");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddCategory"));
    ui.press("escape");
    ui.press("n");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddCategory"));
    ui.press("escape");
    assert_eq!(ui.settings().dialog, None);
}

#[gpui::test]
fn a_new_category_is_added_by_typing_its_name(app: &mut TestAppContext) {
    let mut ui = in_page(app, 5, "Categories");
    let before = ui.settings().category_names.len();

    ui.press("n");
    ui.press("p e t s");
    assert_eq!(ui.settings().dialog_name.as_deref(), Some("pets"));
    ui.press("enter");
    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert_eq!(page.category_names.len(), before + 1);
    assert!(page.category_names.iter().any(|name| name == "pets"));
}

#[gpui::test]
fn a_parent_category_cannot_be_deleted_but_a_leaf_can(app: &mut TestAppContext) {
    let mut ui = in_page(app, 5, "Categories");

    ui.press("j");
    ui.press("d");
    assert_eq!(ui.settings().dialog, None);
    ui.press("right");
    assert_eq!(ui.settings().selected_category.as_deref(), Some("Rent"));
    ui.press("d");
    assert_eq!(ui.settings().dialog.as_deref(), Some("DeleteCategory"));
    ui.press("escape");
    assert_eq!(ui.settings().dialog, None);
}
