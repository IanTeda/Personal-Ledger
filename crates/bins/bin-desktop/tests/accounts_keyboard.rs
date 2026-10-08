//! The Add and Edit account dialogs' form driven by keys: the `Tab` ring, typing into the focused
//! field, the opening balance's filter, the selects (open, step, commit, the first `Esc` closing
//! only the list), the Cash and Edit read-only rules, and `Enter` confirming only a valid form.
//! The dialogs open from the Settings Accounts page; state is read back from `Shell`.

mod common;

use bin_desktop::navigation::nav::{InputMode, Noun};
use common::Harness;
use gpui::TestAppContext;

/// The Settings Accounts page with focus stepped into it.
fn on_accounts(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press("g s");
    assert_eq!(ui.noun(), Noun::Settings);
    ui.press("j j j j");
    assert_eq!(ui.settings().page, "Accounts");
    ui.press("l");
    assert!(ui.settings().page_focused);
    ui
}

fn add_dialog(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = on_accounts(app);
    ui.press("n");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddAccount"));
    ui
}

fn mode(ui: &mut Harness<'_>) -> InputMode {
    ui.read(|shell| shell.nav().mode())
}

#[gpui::test]
fn tab_walks_the_fields_and_wraps_and_shift_tab_walks_back(app: &mut TestAppContext) {
    let mut ui = add_dialog(app);
    assert_eq!(ui.account_form().focused, "Name");

    for expected in [
        "Institution",
        "Type",
        "Unit",
        "OpeningBalance",
        "AccountNumber",
        "Name",
    ] {
        ui.press("tab");
        assert_eq!(ui.account_form().focused, expected);
    }
    ui.press("shift-tab");
    assert_eq!(ui.account_form().focused, "AccountNumber");
    // Still inside the dialog: `Tab` never moves the shell's focus zones behind the scrim.
    assert_eq!(mode(&mut ui), InputMode::Dialog);
}

#[gpui::test]
fn typing_and_backspace_edit_only_the_focused_text_field(app: &mut TestAppContext) {
    let mut ui = add_dialog(app);

    ui.press("a b c");
    ui.press("backspace");
    assert_eq!(ui.account_form().name, "ab");

    ui.press("tab tab tab tab tab");
    assert_eq!(ui.account_form().focused, "AccountNumber");
    ui.press("1 2");
    let form = ui.account_form();
    assert_eq!(form.account_number, "12");
    assert_eq!(form.name, "ab");
    assert_eq!(form.opening_balance, "");
}

#[gpui::test]
fn the_opening_balance_only_takes_a_decimal_amount(app: &mut TestAppContext) {
    let mut ui = add_dialog(app);
    ui.press("tab tab tab tab");
    assert_eq!(ui.account_form().focused, "OpeningBalance");

    ui.press("- 1 a . 5 . - x 2");
    assert_eq!(ui.account_form().opening_balance, "-1.52");
    ui.press("backspace");
    assert_eq!(ui.account_form().opening_balance, "-1.5");
}

#[gpui::test]
fn enter_confirms_only_once_the_form_is_valid(app: &mut TestAppContext) {
    let mut ui = add_dialog(app);
    let before = ui.settings().account_names.len();

    ui.press("enter");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddAccount"));
    assert_eq!(mode(&mut ui), InputMode::Dialog);
    assert!(!ui.account_form().valid);

    ui.press("o k");
    assert!(ui.account_form().valid);
    ui.press("enter");
    let page = ui.settings();
    assert_eq!(page.dialog, None);
    assert_eq!(page.account_names.len(), before + 1);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn enter_on_a_select_opens_it_instead_of_confirming(app: &mut TestAppContext) {
    let mut ui = add_dialog(app);
    ui.press("o k");

    ui.press("tab enter");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddAccount"));
    assert!(ui.account_form().select_open);
}

#[gpui::test]
fn a_select_steps_opens_and_commits_with_the_keys(app: &mut TestAppContext) {
    let mut ui = add_dialog(app);
    ui.press("tab");
    let first = ui.account_form().institution;
    assert!(first.is_some());

    // Closed: down steps the value.
    ui.press("down");
    let second = ui.account_form().institution;
    assert_ne!(first, second);
    ui.press("up");
    assert_eq!(ui.account_form().institution, first);

    // Open: down moves the highlight, enter commits it and closes the list.
    ui.press("space");
    assert!(ui.account_form().select_open);
    ui.press("down enter");
    let form = ui.account_form();
    assert!(!form.select_open);
    assert_eq!(form.institution, second);
}

#[gpui::test]
fn the_first_escape_closes_an_open_list_and_the_next_closes_the_dialog(app: &mut TestAppContext) {
    let mut ui = add_dialog(app);
    ui.press("tab");
    let before = ui.account_form().institution;

    ui.press("enter down");
    assert!(ui.account_form().select_open);
    ui.press("escape");
    let form = ui.account_form();
    assert!(!form.select_open, "the list closed");
    assert_eq!(form.institution, before, "without committing the highlight");
    assert_eq!(ui.settings().dialog.as_deref(), Some("AddAccount"));
    assert_eq!(mode(&mut ui), InputMode::Dialog);

    ui.press("escape");
    assert_eq!(ui.settings().dialog, None);
    assert_eq!(mode(&mut ui), InputMode::Normal);
}

#[gpui::test]
fn tab_commits_an_open_lists_highlight_and_moves_on(app: &mut TestAppContext) {
    let mut ui = add_dialog(app);
    ui.press("tab");
    let before = ui.account_form().institution;

    ui.press("enter down tab");
    let form = ui.account_form();
    assert!(!form.select_open);
    assert_ne!(form.institution, before);
    assert_eq!(form.focused, "Type");
}

#[gpui::test]
fn a_cash_account_has_no_institution_to_pick(app: &mut TestAppContext) {
    let mut ui = add_dialog(app);
    ui.press("tab tab");
    assert_eq!(ui.account_form().focused, "Type");

    // Closed: up steps Bank back to Cash.
    ui.press("up");
    assert_eq!(ui.account_form().account_type.as_deref(), Some("Cash"));

    // Institution is read-only now: `Shift-Tab` from Type skips it.
    ui.press("shift-tab");
    assert_eq!(ui.account_form().focused, "Name");
    ui.press("tab");
    assert_eq!(ui.account_form().focused, "Type");
}

#[gpui::test]
fn editing_fixes_the_unit_and_balance_and_tab_skips_them(app: &mut TestAppContext) {
    let mut ui = on_accounts(app);
    ui.press("e");
    assert_eq!(ui.settings().dialog.as_deref(), Some("EditAccount"));

    // The seeded selection may be Cash (which also skips Institution), so assert the rule that
    // always holds: a full lap never lands on Unit or Opening balance, and wraps to Name.
    let mut seen = Vec::new();
    for _ in 0..4 {
        ui.press("tab");
        seen.push(ui.account_form().focused);
    }
    assert!(seen.contains(&"AccountNumber".to_string()));
    assert!(!seen.contains(&"Unit".to_string()));
    assert!(!seen.contains(&"OpeningBalance".to_string()));
    assert!(seen.contains(&"Name".to_string()), "the ring wraps");
}

#[gpui::test]
fn the_edit_dialog_is_a_form_too_and_enter_saves_it(app: &mut TestAppContext) {
    let mut ui = on_accounts(app);
    let name = ui.settings().selected_account.expect("seeded accounts");

    ui.press("e");
    assert_eq!(ui.account_form().name, name);
    ui.press("backspace");
    ui.press("enter");
    let page = ui.settings();
    assert_eq!(page.dialog, None);
    let mut shorter = name.clone();
    shorter.pop();
    assert!(page.account_names.contains(&shorter));
}
