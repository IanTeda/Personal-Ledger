//! The Categories end-to-end smoke test: the Desktop's real start-up on a temporary SQLite file.
//! Add a Category, restart, see it on the Categories page, and offered in a Payee's Default
//! category select, read from the Categories adapter. Run with the other smoke tests at a parent
//! ticket's close and before a merge to `main` (`mise run end-to-end-ui smoke`).

#![expect(
    clippy::expect_used,
    reason = "test support: a sample row the handover reads should fail the test loudly"
)]

use super::common;
use bin_desktop::shell::router::{SettingsCategoriesSnapshot, SettingsPayeesSnapshot};

use bin_desktop::navigation::nav::Noun;
use common::{Harness, new_ledger};
use gpui::TestAppContext;
use lib_categories::CategoryRepository;
use lib_core::CategoryId;
use lib_database::{BillsStore, Commit, PayeesStore, Storage, TransactionsStore};

/// Settings on the page `presses` down the index, named `page`, with focus stepped into it.
fn on_page(ui: &mut Harness<'_>, presses: &str, page: &str) {
    ui.press("g s");
    ui.press(presses);
    assert_eq!(ui.settings_frame().page, page);
    ui.press("l");
}

/// Whether the Add payee Dialog's Default category select reaches `name`, stepping down its
/// leaves from the first. Settings opens on its first page.
fn default_category_offers(ui: &mut Harness<'_>, name: &str) -> bool {
    on_page(ui, "j j j j j j j j j j", "Payees");
    ui.press("n z o o tab");
    assert_eq!(ui.dialog_value("field").as_deref(), Some("DefaultCategory"));
    let mut offered = false;
    for _ in 0..30 {
        ui.press("down");
        if ui.dialog_value("category").as_deref() == Some(name) {
            offered = true;
            break;
        }
    }
    ui.press("escape");
    offered
}

#[gpui::test]
fn an_added_category_is_there_after_a_restart_and_a_payee_may_default_to_it(
    app: &mut TestAppContext,
) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let config = new_ledger(&dir.path().join("ledger.pldb"));

    let names = {
        let storage = bin_desktop::start_on(&config, common::today())
            .storage()
            .expect("a new Ledger starts");
        let writer = storage.writer.clone();
        let mut ui = Harness::with_storage(app, storage);
        on_page(&mut ui, "j j j j j j j j", "Categories");
        ui.press("n p e t s enter");
        assert_eq!(ui.dialog(), None, "the Add Dialog confirmed");
        let names = ui.settings::<SettingsCategoriesSnapshot>().category_names;
        assert!(names.iter().any(|name| name == "pets"));
        bin_desktop::quit(&writer);
        names
    };

    let storage = bin_desktop::start_on(&config, common::today())
        .storage()
        .expect("the Ledger starts again");
    let mut ui = Harness::with_storage(app, storage);
    assert!(
        default_category_offers(&mut ui, "pets"),
        "the Payees read the new leaf from the Categories adapter"
    );
    // The index keeps its place, two below Categories.
    on_page(&mut ui, "k k", "Categories");
    assert_eq!(
        ui.settings::<SettingsCategoriesSnapshot>().category_names,
        names
    );
}

/// The Id of the Category called `name`.
fn category_id(storage: &Storage, name: &str) -> CategoryId {
    CategoryRepository::categories(&storage.categories)
        .iter()
        .find(|category| category.name == name)
        .map(|category| category.id)
        .expect("the Category is stored")
}

/// How many Splits, Bill Plans and Payee defaults name Category `id`, in the stores.
fn holdings(storage: &Storage, id: CategoryId) -> (usize, usize, usize) {
    let splits = storage
        .transactions
        .all_transactions()
        .iter()
        .flat_map(|transaction| &transaction.splits)
        .filter(|split| split.category_id == id)
        .count();
    let plans = storage
        .bills
        .plans()
        .bill_plans()
        .iter()
        .filter(|plan| plan.category_id == id)
        .count();
    let payees = PayeesStore::payees(&storage.payees)
        .payees()
        .iter()
        .filter(|payee| payee.default_category == Some(id))
        .count();
    (splits, plans, payees)
}

/// Every Payee's name with the Default category its Edit Dialog shows, read down the list.
fn payee_defaults(ui: &mut Harness<'_>) -> Vec<(String, Option<String>)> {
    on_page(ui, "j j j j j j j j j j", "Payees");
    ui.press("g g");
    let mut defaults: Vec<(String, Option<String>)> = Vec::new();
    // `j` stops on the last row, so the walk ends where a row repeats.
    for _ in 0..100 {
        ui.press("j");
        let name = ui
            .settings::<SettingsPayeesSnapshot>()
            .selected_payee
            .expect("a Payee is selected");
        if defaults.last().is_some_and(|(last, _)| *last == name) {
            break;
        }
        ui.press("e");
        let shown = ui.dialog_value("category");
        ui.press("escape");
        defaults.push((name, shown));
    }
    defaults
}

/// The names of the Payees whose Default category reads `category`.
fn payees_defaulting_to(defaults: &[(String, Option<String>)], category: &str) -> Vec<String> {
    defaults
        .iter()
        .filter(|(_, shown)| shown.as_deref() == Some(category))
        .map(|(name, _)| name.clone())
        .collect()
}

/// The Transactions page's CATEGORY cells, and the Bills Planner's, as they read now.
fn category_cells(ui: &mut Harness<'_>) -> (Vec<String>, Vec<String>) {
    ui.press("g l");
    assert_eq!(ui.noun(), Noun::Transactions);
    let transactions = ui.transactions().categories;
    ui.press("g w");
    assert_eq!(ui.noun(), Noun::Bills);
    (transactions, ui.bills().plan_categories)
}

/// Moves the Categories page's selection to the row called `name`.
fn select(ui: &mut Harness<'_>, name: &str) {
    for _ in 0..20 {
        if ui
            .settings::<SettingsCategoriesSnapshot>()
            .selected_category
            .as_deref()
            == Some(name)
        {
            break;
        }
        ui.press("j");
    }
    assert_eq!(
        ui.settings::<SettingsCategoriesSnapshot>()
            .selected_category
            .as_deref(),
        Some(name)
    );
}

/// The count of `name` among `cells`.
fn count(cells: &[String], name: &str) -> usize {
    cells.iter().filter(|cell| *cell == name).count()
}

#[gpui::test]
fn a_leafs_first_child_keeps_its_splits_bill_plans_and_payee_defaults_after_a_restart(
    app: &mut TestAppContext,
) {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let config = new_ledger(&dir.path().join("ledger.pldb"));

    let (groceries, splits, split_cells, plan_rows, payees_before) = {
        let mut storage = bin_desktop::start_on(&config, common::today())
            .storage()
            .expect("a new Ledger starts");
        let writer = storage.writer.clone();
        // The sample files no Bill Plan under Groceries, so file one: the Telstra Internet plan.
        let groceries = category_id(&storage, "Groceries");
        let mut plan = storage
            .bills
            .plans()
            .bill_plans()
            .into_iter()
            .find(|plan| plan.name == "Telstra Internet")
            .expect("the sample's Telstra Internet plan");
        plan.category_id = groceries;
        storage.bills.plans_mut().upsert_bill_plan(plan);
        storage.pending.commit();
        let (splits, plans, payees) = holdings(&storage, groceries);
        assert!(
            splits > 0 && plans > 0 && payees > 0,
            "Groceries holds all three"
        );

        let mut ui = Harness::with_storage(app, storage);
        let (transactions, planner) = category_cells(&mut ui);
        let split_rows = count(&transactions, "Groceries");
        let plan_rows = count(&planner, "Groceries");
        assert!(split_rows > 0 && plan_rows > 0);
        assert_eq!(count(&transactions, "Supermarket"), 0);
        let defaults = payee_defaults(&mut ui);
        let payees_before = payees_defaulting_to(&defaults, "Groceries");
        assert!(!payees_before.is_empty());

        on_page(&mut ui, "k k", "Categories");
        select(&mut ui, "Groceries");
        ui.press("shift-n");
        ui.press("shift-s u p e r m a r k e t");
        assert_eq!(ui.dialog_value("name").as_deref(), Some("Supermarket"));
        ui.press("enter");
        assert_eq!(ui.dialog(), None, "the Add Dialog confirmed");
        bin_desktop::quit(&writer);
        (groceries, splits, split_rows, plan_rows, payees_before)
    };

    let storage = bin_desktop::start_on(&config, common::today())
        .storage()
        .expect("the Ledger starts again");
    let supermarket = category_id(&storage, "Supermarket");
    assert_eq!(
        holdings(&storage, groceries),
        (0, 0, 0),
        "Groceries holds nothing"
    );
    assert_eq!(
        holdings(&storage, supermarket).0,
        splits,
        "every Split moved to Supermarket"
    );
    assert_eq!(holdings(&storage, supermarket).1, 1, "the Bill Plan moved");
    assert_eq!(
        holdings(&storage, supermarket).2,
        payees_before.len(),
        "every Payee default moved"
    );

    let mut ui = Harness::with_storage(app, storage);
    let (transactions, planner) = category_cells(&mut ui);
    assert_eq!(count(&transactions, "Groceries"), 0);
    assert_eq!(count(&transactions, "Supermarket"), split_cells);
    assert_eq!(count(&planner, "Groceries"), 0);
    assert_eq!(count(&planner, "Supermarket"), plan_rows);
    let defaults = payee_defaults(&mut ui);
    assert!(payees_defaulting_to(&defaults, "Groceries").is_empty());
    assert_eq!(
        payees_defaulting_to(&defaults, "Supermarket"),
        payees_before
    );
}
