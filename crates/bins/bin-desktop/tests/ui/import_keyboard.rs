//! The Import step driven by keys: `:import` opening it in place of Transactions, row movement,
//! Remember, the Payee and Category selects, committing, and leaving it. State is read back from
//! `Root`.

use super::common;
use bin_desktop::shell::router::{SettingsAccountsSnapshot, SettingsPayeesSnapshot};

use bin_desktop::navigation::nav::Noun;
use common::Harness;
use gpui::TestAppContext;

fn on_import(app: &mut TestAppContext) -> Harness<'_> {
    let mut ui = Harness::new(app);
    ui.press(": i m p o r t enter");
    assert_eq!(ui.noun(), Noun::Transactions);
    ui
}

#[gpui::test]
fn import_opens_on_the_seeded_statement_in_place_of_transactions(app: &mut TestAppContext) {
    let mut ui = on_import(app);

    let import = ui.import().expect("`:import` opens the step");
    assert_eq!(import.rows.len(), 18);
    assert_eq!(import.selected, 0);
    assert!(import.remember);
    assert_eq!(import.open_select, None);
    assert!(
        !import.can_continue,
        "unmatched rows need review before continuing"
    );
}

#[gpui::test]
fn j_and_k_move_the_row_and_r_toggles_remember(app: &mut TestAppContext) {
    let mut ui = on_import(app);

    ui.press("j j");
    assert_eq!(ui.import().expect("import is open").selected, 2);
    ui.press("k");
    assert_eq!(ui.import().expect("import is open").selected, 1);
    ui.press("r");
    assert!(!ui.import().expect("import is open").remember);
    ui.press("r");
    assert!(ui.import().expect("import is open").remember);
}

#[gpui::test]
fn p_opens_the_payee_select_and_escape_closes_only_the_select(app: &mut TestAppContext) {
    let mut ui = on_import(app);

    ui.press("p");
    assert_eq!(
        ui.import().expect("import is open").open_select.as_deref(),
        Some("Payee")
    );
    ui.press("escape");
    let import = ui.import().expect("escape closed the select, not the step");
    assert_eq!(import.open_select, None);
}

#[gpui::test]
fn c_opens_the_category_select(app: &mut TestAppContext) {
    let mut ui = on_import(app);

    ui.press("c");
    assert_eq!(
        ui.import().expect("import is open").open_select.as_deref(),
        Some("Category")
    );
}

#[gpui::test]
fn enter_does_nothing_while_a_row_needs_review(app: &mut TestAppContext) {
    let mut ui = on_import(app);

    ui.press("enter");
    assert!(ui.import().is_some());
    assert!(ui.toasts().history.is_empty());
}

#[gpui::test]
fn escape_leaves_the_import_step(app: &mut TestAppContext) {
    let mut ui = on_import(app);

    ui.press("escape");
    assert_eq!(ui.import(), None);
    assert_eq!(ui.noun(), Noun::Transactions);
}

/// Moves the Import selection to row `index` with `j` and `k`.
#[expect(
    clippy::expect_used,
    reason = "test support: an Import step that isn't open should fail the test loudly"
)]
fn select_row(ui: &mut Harness<'_>, index: usize) {
    let selected = ui.import().expect("import is open").selected;
    let (key, presses) = if index >= selected {
        ("j", index - selected)
    } else {
        ("k", selected - index)
    };
    for _ in 0..presses {
        ui.press(key);
    }
    assert_eq!(ui.import().expect("import is open").selected, index);
}

/// Opens Import, takes the candle shop's row as a new Payee, resolves every other row and commits,
/// so one Split names a Payee the commit created. Ends on Transactions with the Import closed.
#[expect(
    clippy::expect_used,
    reason = "test support: an Import step that won't commit should fail the test loudly"
)]
pub(super) fn commit_the_candle_shop_as_a_new_payee(ui: &mut Harness<'_>) {
    ui.press(": i m p o r t enter");
    assert_eq!(ui.noun(), Noun::Transactions);
    let candle = ui
        .import()
        .expect("import is open")
        .rows
        .iter()
        .position(|row| row.raw == "SP AUSSIE CANDLE CO")
        .expect("the seeded statement has the candle shop");
    select_row(ui, candle);
    ui.press("n");

    let to_review: Vec<usize> = ui
        .import()
        .expect("import is open")
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.status == "NeedsReview")
        .map(|(index, _)| index)
        .collect();
    for index in to_review {
        select_row(ui, index);
        // `j` steps past "choose category…" to the first leaf.
        ui.press("c j enter");
    }
    let import = ui.import().expect("import is open");
    assert!(import.remember);
    assert!(import.can_continue, "every row is resolved");
    assert_eq!(import.rows[candle].status, "NewPayee");

    let before = ui.commits();
    ui.press("enter");
    assert_eq!(ui.import(), None, "enter committed the import");
    assert_eq!(ui.commits(), before + 1, "one commit for the input");
    let toast = ui
        .toasts()
        .history
        .first()
        .cloned()
        .expect("a Toast says so");
    assert_eq!(toast.kind, "Success");
    assert_eq!(toast.text, "Imported 18 transactions");
}

/// A Payee booked with the Import's new name after the step opened refuses the commit: the Toast
/// is an Error naming the clash in a Message, and the step stays open.
#[gpui::test]
fn a_refused_commit_raises_an_error_toast_naming_the_clash(app: &mut TestAppContext) {
    let mut ui = on_import(app);
    let candle = ui
        .import()
        .expect("import is open")
        .rows
        .iter()
        .position(|row| row.raw == "SP AUSSIE CANDLE CO")
        .expect("the seeded statement has the candle shop");
    select_row(&mut ui, candle);
    ui.press("n");
    let to_review: Vec<usize> = ui
        .import()
        .expect("import is open")
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.status == "NeedsReview")
        .map(|(index, _)| index)
        .collect();
    for index in to_review {
        select_row(&mut ui, index);
        ui.press("c j enter");
    }
    ui.book_payee("Aussie Candle Co");

    ui.press("enter");

    assert!(
        ui.import().is_some(),
        "a refused commit leaves the step open"
    );
    let toast = ui
        .toasts()
        .history
        .first()
        .cloned()
        .expect("a Toast says so");
    assert_eq!(toast.kind, "Error");
    assert_eq!(toast.text, "Aussie Candle Co already exists");
}

/// The Import View and the Transactions list are separate Entities, so the commit's landing on
/// the list's first row is a hand-off between them.
#[gpui::test]
fn a_commit_lands_on_the_transactions_list_from_its_first_row(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    ui.press("g l");
    ui.press("j j j");
    assert_eq!(ui.transactions().selected, 3);

    commit_the_candle_shop_as_a_new_payee(&mut ui);

    assert_eq!(ui.noun(), Noun::Transactions);
    assert_eq!(ui.transactions().selected, 0);
}

#[gpui::test]
fn a_committed_new_payee_shows_on_the_payees_page_with_its_remembered_rule(
    app: &mut TestAppContext,
) {
    let mut ui = Harness::new(app);
    commit_the_candle_shop_as_a_new_payee(&mut ui);

    ui.press("g s");
    ui.press("j j j j j j j");
    assert_eq!(ui.settings_frame().page, "Payees");
    let page = ui.settings::<SettingsPayeesSnapshot>();
    let row = page
        .payee_names
        .iter()
        .position(|name| name == "Aussie Candle Co")
        .expect("the Import commit's new Payee is listed");
    assert_eq!(page.payee_rules[row], "1 rule");
}

/// The Transactions count ANZ Everyday's Edit and Delete Dialogs show, `(edit, delete)`, opening
/// each from its row on Settings › Accounts and cancelling it. Settings reopens on the page it
/// was left on, so the index is walked only the first time.
#[expect(
    clippy::expect_used,
    reason = "test support: a Dialog that doesn't read back its count should fail the test loudly"
)]
fn everyday_dialog_counts(ui: &mut Harness<'_>) -> (usize, usize) {
    ui.press("g s");
    if ui.settings_frame().page != "Accounts" {
        ui.press("j j j j");
    }
    assert_eq!(ui.settings_frame().page, "Accounts");
    ui.press("l");
    ui.press("g g j");
    assert_eq!(
        ui.settings::<SettingsAccountsSnapshot>()
            .selected_account
            .as_deref(),
        Some("ANZ Everyday")
    );
    let mut count = |key: &str, dialog: &str| {
        ui.press(key);
        assert_eq!(ui.dialog(), Some(dialog));
        let transactions = ui
            .dialog_value("transactions")
            .expect("the Dialog reads back its count")
            .parse()
            .expect("a count");
        ui.press("escape");
        transactions
    };
    let edit = count("e", "EditAccount");
    let delete = count("d", "DeleteAccount");
    ui.press("h");
    (edit, delete)
}

/// An Import's Transactions show in their Account's Edit and Delete Dialogs, counted from the
/// Transactions with nothing writing the Account: the commit is one write, the booking (#852).
#[gpui::test]
fn an_imports_transactions_show_in_their_accounts_edit_and_delete_counts(app: &mut TestAppContext) {
    let mut ui = Harness::new(app);
    let (edit, delete) = everyday_dialog_counts(&mut ui);
    assert_eq!(edit, delete);

    commit_the_candle_shop_as_a_new_payee(&mut ui);

    assert_eq!(everyday_dialog_counts(&mut ui), (edit + 18, delete + 18));
}
