//! The Categories ports as the Desktop holds them (#858, #859): the store `shell::router::build` is
//! given, shared by the Categories module, Category delete (`crate::category_deletion`, #1005), the
//! Ledger's read copy, the Bills ports (`BillCategories`) and the Payees ports and the Import's
//! Category select (`PayeeCategories`, `ImportCategories`). The writes are committed by the input
//! they were made in (`crate::input::send`), not here.
//!
//! The Categories page writes the repository through `lib-categories`' Interactors. What the
//! module reaches of other domains goes through [`LedgerPorts`]: the usage its Dialogs quote (the
//! Splits from the Transactions store, #926, the Budgets from the Budgets store, #952), the
//! Monthly budget the Add and Edit Dialogs read and save in the Personal spending Budget, on the
//! Budgets store (#952), and the handover of a leaf's rows to its first child, which the module
//! writes to the Transactions, Bills and Payees stores itself ([`LedgerPorts`]'s `hand_over`).

use std::cell::RefCell;
use std::rc::Rc;

use chrono::{Datelike as _, NaiveDate};
use gpui::{App, Entity};
use lib_budgets::{Budgets, Error};
use lib_categories::{
    Category, CategoryBudget, CategoryHandover, CategoryRepository, CategoryUsage, CategoryUse,
    Holdings, MonthlyBudget, MonthlyBudgetRefusal, ParentStop, SharedCategories,
    TypeChangeBlockers,
};
use lib_core::{CategoryId, Money};
use lib_database::{PayeesStore, SharedCategoriesStore};
use lib_ledger::Ledger;

use crate::budgets::PERSONAL_SPENDING_ID;
use crate::ledger_store::LedgerStore;

/// `ledger`'s own Categories in memory, for a test that needs no storage.
pub fn in_memory(ledger: &Ledger) -> SharedCategoriesStore {
    Rc::new(RefCell::new(lib_categories::fake::InMemoryCategories {
        categories: ledger.categories().to_vec(),
    }))
}

/// The shared store as the module's Presenters hold the repository, borrowed for each call: the
/// store is shared behind a `RefCell` whose guard can't be lent out of it.
pub struct StoreCategories(SharedCategoriesStore);

impl CategoryRepository for StoreCategories {
    fn categories(&self) -> Vec<Category> {
        self.0.borrow().categories().categories()
    }

    fn upsert_category(&mut self, category: Category) {
        self.0
            .borrow_mut()
            .categories_mut()
            .upsert_category(category);
    }
}

/// `store` as the Categories module's Presenters share it.
pub fn shared(store: &SharedCategoriesStore) -> SharedCategories {
    Rc::new(RefCell::new(StoreCategories(store.clone())))
}

/// Brings the Ledger read copy of the Categories in line with the store, announcing
/// `Domain::Categories` when it changed. Run before and after every Categories event and Dialog
/// confirm, so Transactions, Budgets, Import and the Dashboard see a write at once.
pub fn refresh(ledger: &Entity<LedgerStore>, cx: &mut App) {
    ledger.update(cx, |store, cx| store.refresh_categories(cx));
}

/// The Categories ports the Desktop answers from its Ledger, built for one call around its gpui
/// context: what names a Category, the Monthly budget in the Personal spending Budget as of
/// `today` (the day the page was built), and the handover.
pub struct LedgerPorts<'a> {
    pub ledger: &'a Entity<LedgerStore>,
    pub today: NaiveDate,
    pub cx: &'a mut App,
}

impl LedgerPorts<'_> {
    /// The Budgets as the Budgets store holds them.
    fn budgets(&self) -> Budgets {
        Budgets::from_stored(
            self.ledger
                .read(self.cx)
                .budgets_store()
                .borrow()
                .budgets()
                .budgets(),
        )
    }
}

impl CategoryUsage for LedgerPorts<'_> {
    /// The Splits counted by the Transactions store (#926), and the Budgets holding a Category
    /// Limit for it by the Budgets store (#952).
    fn category_usage(&self, id: CategoryId) -> CategoryUse {
        let ledger_store = self.ledger.read(self.cx);
        CategoryUse {
            splits: ledger_store.category_splits(id),
            budgets: ledger_store.budgets_store().borrow().category_budgets(id),
        }
    }

    /// The active Budgets using the subtree this month or later, and every Bill Plan naming it.
    fn type_change_blockers(&self, subtree: &[CategoryId]) -> TypeChangeBlockers {
        TypeChangeBlockers {
            budgets: self.budgets().using(subtree, self.today),
            bill_plans: self.ledger.read(self.cx).bill_plans_naming(subtree),
        }
    }
}

/// What a refused Budget write means to the Categories Dialogs.
fn budget_refusal(error: Error) -> MonthlyBudgetRefusal {
    match error {
        Error::NotAnExpenseLeaf => MonthlyBudgetRefusal::NotAnExpenseLeaf,
        Error::Archived | Error::ClosedMonth => MonthlyBudgetRefusal::ReadOnly,
        _ => MonthlyBudgetRefusal::Unavailable,
    }
}

impl CategoryBudget for LedgerPorts<'_> {
    fn monthly_budget(
        &self,
        categories: &[Category],
        category: Option<CategoryId>,
    ) -> MonthlyBudget {
        let budgets = self.budgets();
        MonthlyBudget {
            amount: category.and_then(|id| {
                budgets.monthly_limit(
                    PERSONAL_SPENDING_ID,
                    &lib_ledger::budget_categories(categories),
                    id,
                    self.today,
                )
            }),
            archived: budgets
                .get(PERSONAL_SPENDING_ID)
                .filter(|budget| budget.is_archived())
                .map(|budget| budget.name.clone()),
        }
    }

    /// Runs the save on a copy of the Budgets, so the check is the Budgets' own rules.
    fn check_monthly_budget(
        &self,
        categories: &[Category],
        category: CategoryId,
        amount: Option<&Money>,
    ) -> Result<(), MonthlyBudgetRefusal> {
        let mut budgets = self.budgets();
        budgets
            .set_monthly_limit(
                PERSONAL_SPENDING_ID,
                &lib_ledger::budget_categories(categories),
                category,
                amount.cloned(),
                self.today,
            )
            .map(|_| ())
            .map_err(budget_refusal)
    }

    /// Saves the amount to the Budgets store on the Categories store's tree, which holds the
    /// Category just written, then brings the Budgets read copy in line.
    fn set_monthly_budget(
        &mut self,
        category: CategoryId,
        amount: Option<Money>,
    ) -> Result<(), MonthlyBudgetRefusal> {
        let today = self.today;
        self.ledger.update(self.cx, |store, cx| {
            store.refresh_categories(cx);
            let categories = store.budget_categories();
            let saved = crate::budgets::ports::set_monthly_limit(
                &mut *store.budgets_store().borrow_mut(),
                &categories,
                PERSONAL_SPENDING_ID,
                category,
                amount,
                today,
            );
            store.refresh_budgets(cx);
            saved.map(|_| ()).map_err(budget_refusal)
        })
    }

    fn parent_stop(&self, category: CategoryId) -> Option<ParentStop> {
        let budgets = self.budgets().budgeting(category, self.today);
        (budgets > 0).then_some(ParentStop {
            budgets,
            month: self.today.month(),
        })
    }

    /// Stops the parent in every active Budget in the Budgets store, in the input's one commit,
    /// then brings the Budgets read copy in line.
    fn stop_new_parent(&mut self, parent: CategoryId) {
        let today = self.today;
        self.ledger.update(self.cx, |store, cx| {
            crate::budgets::ports::stop_new_parent(
                &mut *store.budgets_store().borrow_mut(),
                parent,
                today,
            );
            store.refresh_budgets(cx);
        });
    }
}

impl CategoryHandover for LedgerPorts<'_> {
    fn holdings(&self, category: CategoryId) -> Holdings {
        self.ledger.read(self.cx).category_holdings(category)
    }

    /// Re-points the rows in the Transactions, Bills and Payees stores, in the input's one commit,
    /// then brings their read copies in line and announces the Payees.
    fn hand_over(&mut self, from: CategoryId, to: CategoryId) {
        self.ledger.update(self.cx, |store, cx| {
            hand_over_rows(store, from, to);
            store.refresh_transactions(cx);
            store.refresh_bills(cx);
            store.announce_payees(cx);
        });
    }
}

/// Points every Split, Bill Plan and Payee default naming `from` at `to`, in the stores `store`
/// lends: the Categories module writes them, not the Ledger store (#1006).
fn hand_over_rows(store: &LedgerStore, from: CategoryId, to: CategoryId) {
    {
        let mut transactions = store.transactions_store().borrow_mut();
        for mut transaction in transactions.all_transactions() {
            if transaction
                .splits
                .iter()
                .all(|split| split.category_id != from)
            {
                continue;
            }
            for split in &mut transaction.splits {
                if split.category_id == from {
                    split.category_id = to;
                }
            }
            transactions.upsert_transaction(transaction);
        }
    }
    {
        let bills = store.bills_store();
        let mut bills = bills.borrow_mut();
        for mut plan in bills.plans().bill_plans() {
            if plan.category_id == from {
                plan.category_id = to;
                bills.plans_mut().upsert_bill_plan(plan);
            }
        }
    }
    let mut payees = store.payees_ports().borrow_mut();
    for mut payee in PayeesStore::payees(&*payees).payees() {
        if payee.default_category == Some(from) {
            payee.default_category = Some(to);
            PayeesStore::payees_mut(&mut *payees).upsert_payee(payee);
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, TestAppContext};

    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 11).expect("a real date")
    }

    #[gpui::test]
    fn the_budget_port_checks_on_a_copy_and_saves_through_the_ledger(cx: &mut TestAppContext) {
        let ledger = cx.new(|_| LedgerStore::new(Ledger::stub_on(today())));
        let categories = ledger.read_with(cx, |store, _| store.ledger().categories().to_vec());
        let groceries = lib_categories::find_by_name(&categories, "Groceries").expect("seeded");
        let salary = lib_categories::find_by_name(&categories, "Salary").expect("seeded");
        let amount: Money = "612".parse().expect("an amount");

        cx.update(|cx| {
            let mut ports = LedgerPorts {
                ledger: &ledger,
                today: today(),
                cx,
            };
            assert_eq!(
                ports.check_monthly_budget(&categories, salary, Some(&amount)),
                Err(MonthlyBudgetRefusal::NotAnExpenseLeaf)
            );
            assert_eq!(
                ports.check_monthly_budget(&categories, groceries, Some(&amount)),
                Ok(())
            );
            assert_ne!(
                ports.monthly_budget(&categories, Some(groceries)).amount,
                Some(amount.clone()),
                "a check writes nothing"
            );
            ports
                .set_monthly_budget(groceries, Some(amount.clone()))
                .expect("a leaf Expense");
            assert_eq!(
                ports.monthly_budget(&categories, Some(groceries)).amount,
                Some(amount)
            );
        });
    }

    #[gpui::test]
    fn a_new_parent_is_stopped_in_every_active_budget_through_the_store(cx: &mut TestAppContext) {
        let ledger = cx.new(|_| LedgerStore::new(Ledger::stub_on(today())));
        let categories = ledger.read_with(cx, |store, _| store.ledger().categories().to_vec());
        let groceries = lib_categories::find_by_name(&categories, "Groceries").expect("seeded");

        cx.update(|cx| {
            let mut ports = LedgerPorts {
                ledger: &ledger,
                today: today(),
                cx,
            };
            let stop = ports.parent_stop(groceries).expect("budgeted in the seed");
            assert_eq!((stop.budgets, stop.month), (2, 10));
            assert!(
                ports
                    .monthly_budget(&categories, Some(groceries))
                    .amount
                    .is_some()
            );

            ports.stop_new_parent(groceries);

            assert_eq!(ports.parent_stop(groceries), None);
            assert_eq!(
                ports.monthly_budget(&categories, Some(groceries)).amount,
                None
            );
        });
    }

    #[gpui::test]
    fn the_handover_port_moves_splits_to_the_new_child_through_the_stores(cx: &mut TestAppContext) {
        let ledger = cx.new(|_| LedgerStore::new(Ledger::stub_on(today())));
        let categories = ledger.read_with(cx, |store, _| store.ledger().categories().to_vec());
        let groceries = lib_categories::find_by_name(&categories, "Groceries").expect("seeded");
        let child = lib_categories::find_by_name(&categories, "Dining")
            .expect("another seeded leaf stands in for a new child");

        cx.update(|cx| {
            let mut ports = LedgerPorts {
                ledger: &ledger,
                today: today(),
                cx,
            };
            let held = ports.holdings(groceries);
            assert!(held.splits > 0, "the stub buys groceries");
            let child_before = ports.holdings(child);

            ports.hand_over(groceries, child);

            assert_eq!(ports.holdings(groceries), Holdings::default());
            assert_eq!(
                ports.holdings(child).splits,
                child_before.splits + held.splits
            );
        });
    }
}
