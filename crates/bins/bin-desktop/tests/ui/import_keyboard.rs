//! The Import step driven by keys: `:import` opening it in place of Transactions, row movement,
//! Remember, the Payee and Category selects, committing, and leaving it. State is read back from
//! `Root`.

use super::common;

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

#[gpui::test]
fn a_committed_new_payee_shows_on_the_payees_page_with_its_remembered_rule(
    app: &mut TestAppContext,
) {
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

    ui.press("g s");
    ui.press("j j j j j j j");
    assert_eq!(ui.settings().page, "Payees");
    let page = ui.settings_payees();
    let row = page
        .payee_names
        .iter()
        .position(|name| name == "Aussie Candle Co")
        .expect("the Import commit's new Payee is listed");
    assert_eq!(page.payee_rules[row], "1 rule");
}
