//! The Categories ports as the Desktop holds them (#858, #859): the store `shell::router::build` is
//! given, shared by the Categories module, the Ledger's forwarding shim and read copy, the Bills
//! ports (`BillCategories`) and the Payees ports and Import's Category select (`PayeeCategories`).
//! The writes are committed by the input they were made in (`crate::input::send`), not here.
//!
//! The Categories page writes the repository through `lib-categories`' Interactors. What the
//! module reaches of domains not yet persisted goes through the Ledger, in [`LedgerPorts`]: the
//! usage its Dialogs quote, the Monthly budget the Add and Edit Dialogs save into the Personal
//! spending Budget (#811), and Category delete, the reassign port, answered by running
//! `lib_ledger::DeleteCategory` through `LedgerStore::apply`, whose shims carry its writes to the
//! Categories, Payees, Transactions and Bills stores in the one commit (Q6, until `lib-reassign`,
//! #813). [`forward`] and [`forward_deletes`] are the Categories shim's two halves.

use std::cell::RefCell;
use std::rc::Rc;

use chrono::NaiveDate;
use gpui::{App, Entity};
use lib_budgets::BudgetError;
use lib_categories::{
    Category, CategoryBudget, CategoryReassign, CategoryReassigned, CategoryRepository,
    CategoryUsage, CategoryUse, MonthlyBudget, MonthlyBudgetRefusal, ReassignRefusal,
    SharedCategories,
};
use lib_core::{CategoryId, Money};
use lib_database::{CategoriesStore, SharedCategoriesStore};
use lib_ledger::{
    DeleteCategory, DeleteCategoryRefusal, Ledger, LedgerChange, SetCategoryMonthlyLimit,
};

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

/// The forwarding shim's first half (#858): sends each Category row `changes` upserts to `store`,
/// in the order recorded, returning whether it wrote anything. Run before the Payees, Transactions
/// and Bills, since Category reassign re-points their rows to an Uncategorised it may create.
/// Writer carried: Category reassign's Uncategorised, until `lib-reassign` (#813); the Categories
/// page writes the port itself (#859).
pub fn forward(store: &mut dyn CategoriesStore, changes: &[LedgerChange]) -> bool {
    let port = store.categories_mut();
    let mut wrote = false;
    for change in changes {
        if let LedgerChange::UpsertCategory(category) = change {
            port.upsert_category(category.clone());
            wrote = true;
        }
    }
    wrote
}

/// The forwarding shim's second half: sends each Category `changes` deletes to `store`, returning
/// whether it wrote anything. Run after the Payees, Transactions and Bills, since the rows naming
/// the Category must be re-pointed before it goes (their foreign keys). Writer carried: Category
/// reassign, until `lib-reassign` (#813).
pub fn forward_deletes(store: &mut dyn CategoriesStore, changes: &[LedgerChange]) -> bool {
    let mut wrote = false;
    for change in changes {
        if let LedgerChange::DeleteCategory(id) = change {
            store.delete_category(*id);
            wrote = true;
        }
    }
    wrote
}

/// Brings the Ledger read copy of the Categories in line with the store, announcing
/// `Domain::Categories` when it changed. Run before and after every Categories event and Dialog
/// confirm, so Transactions, Budgets, Import and the Dashboard see a write at once, and a reassign
/// runs on the rows the page shows.
pub fn refresh(ledger: &Entity<LedgerStore>, cx: &mut App) {
    ledger.update(cx, |store, cx| store.refresh_categories(cx));
}

/// The Categories ports the Desktop answers from its Ledger, built for one call around its gpui
/// context: what names a Category, the Monthly budget in the Personal spending Budget as of
/// `today` (the day the page was built), and Category delete.
pub struct LedgerPorts<'a> {
    pub ledger: &'a Entity<LedgerStore>,
    pub today: NaiveDate,
    pub cx: &'a mut App,
}

impl LedgerPorts<'_> {
    fn ledger(&self) -> &Ledger {
        self.ledger.read(self.cx).ledger()
    }
}

impl CategoryUsage for LedgerPorts<'_> {
    /// The Splits counted from the Ledger's read copy of the Transactions, and the Budgets holding
    /// a Category Limit for it from its Budgets.
    fn category_usage(&self, id: CategoryId) -> CategoryUse {
        let ledger = self.ledger();
        CategoryUse {
            splits: ledger
                .transactions()
                .iter()
                .flat_map(|transaction| &transaction.splits)
                .filter(|split| split.category_id == id)
                .count(),
            budgets: ledger.budgets().limit_count(id),
        }
    }
}

/// What a refused Budget write means to the Categories Dialogs.
fn budget_refusal(error: BudgetError) -> MonthlyBudgetRefusal {
    match error {
        BudgetError::NotAnExpenseLeaf => MonthlyBudgetRefusal::NotAnExpenseLeaf,
        BudgetError::Archived | BudgetError::ClosedMonth => MonthlyBudgetRefusal::ReadOnly,
        _ => MonthlyBudgetRefusal::Unavailable,
    }
}

impl CategoryBudget for LedgerPorts<'_> {
    fn monthly_budget(
        &self,
        categories: &[Category],
        category: Option<CategoryId>,
    ) -> MonthlyBudget {
        let budgets = self.ledger().budgets();
        MonthlyBudget {
            amount: category.and_then(|id| {
                budgets.monthly_limit(PERSONAL_SPENDING_ID, categories, id, self.today)
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
        let mut budgets = self.ledger().budgets().clone();
        budgets
            .set_monthly_limit(
                PERSONAL_SPENDING_ID,
                categories,
                category,
                amount.cloned(),
                self.today,
            )
            .map(|_| ())
            .map_err(budget_refusal)
    }

    /// Brings the read copy in line with the Category just written, then saves the amount through
    /// `LedgerStore::apply`.
    fn set_monthly_budget(
        &mut self,
        category: CategoryId,
        amount: Option<Money>,
    ) -> Result<(), MonthlyBudgetRefusal> {
        let today = self.today;
        self.ledger.update(self.cx, |store, cx| {
            store.refresh_categories(cx);
            store
                .apply(
                    cx,
                    SetCategoryMonthlyLimit {
                        id: category,
                        monthly: lib_ledger::MonthlyBudget {
                            budget_id: PERSONAL_SPENDING_ID,
                            today,
                            limit: amount,
                        },
                    },
                )
                .map(|_| ())
                .map_err(budget_refusal)
        })
    }
}

impl CategoryReassign for LedgerPorts<'_> {
    /// Today's `LedgerCommand`, on the read copy brought in line with the store first.
    fn delete_category(&mut self, id: CategoryId) -> Result<CategoryReassigned, ReassignRefusal> {
        self.ledger.update(self.cx, |store, cx| {
            store.refresh_categories(cx);
            store
                .apply(cx, DeleteCategory(id))
                .map(|deleted| CategoryReassigned {
                    name: deleted.name,
                    splits_moved: deleted.splits_moved,
                })
                .map_err(|refusal| match refusal {
                    DeleteCategoryRefusal::NotFound => ReassignRefusal::NotFound,
                    DeleteCategoryRefusal::HasChildren => ReassignRefusal::HasChildren,
                    DeleteCategoryRefusal::UncategorisedInUse => {
                        ReassignRefusal::UncategorisedInUse
                    }
                })
        })
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, TestAppContext};
    use lib_core::TagId;

    use super::*;

    #[test]
    fn category_upserts_and_deletes_reach_the_store_in_their_own_halves() {
        let ledger = Ledger::stub();
        let store = in_memory(&ledger);
        let mut groceries = ledger.categories()[6].clone();
        groceries.name = "Supermarket".into();
        let gone = CategoryId::seed(8);
        let changes = [
            LedgerChange::UpsertCategory(groceries.clone()),
            LedgerChange::DeleteCategory(gone),
            LedgerChange::DeleteTag(TagId::seed(1)),
        ];

        assert!(forward(&mut *store.borrow_mut(), &changes));
        assert!(
            store.borrow().categories().category(gone).is_some(),
            "not yet"
        );
        assert!(forward_deletes(&mut *store.borrow_mut(), &changes));

        let store = store.borrow();
        assert_eq!(store.categories().category(groceries.id), Some(groceries));
        assert!(store.categories().category(gone).is_none());
        assert!(!forward(
            &mut lib_categories::fake::InMemoryCategories::default(),
            &changes[2..]
        ));
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 11).expect("a real date")
    }

    #[gpui::test]
    fn the_reassign_port_moves_the_splits_and_deletes_through_the_ledger(cx: &mut TestAppContext) {
        let ledger = cx.new(|_| LedgerStore::new(Ledger::stub_on(today())));
        let groceries = lib_categories::find_by_name(
            ledger
                .read_with(cx, |store, _| store.ledger().categories().to_vec())
                .as_slice(),
            "Groceries",
        )
        .expect("seeded");

        let (uses, deleted) = cx.update(|cx| {
            let mut ports = LedgerPorts {
                ledger: &ledger,
                today: today(),
                cx,
            };
            (
                ports.category_usage(groceries),
                ports.delete_category(groceries),
            )
        });

        let deleted = deleted.expect("Groceries is a leaf");
        assert_eq!(deleted.name, "Groceries");
        assert_eq!(deleted.splits_moved, uses.splits);
        assert!(uses.splits > 0, "the stub buys groceries");
        ledger.read_with(cx, |store, _| {
            assert!(
                lib_categories::find_by_name(store.ledger().categories(), "Groceries").is_none()
            );
        });
        let refusal = cx.update(|cx| {
            LedgerPorts {
                ledger: &ledger,
                today: today(),
                cx,
            }
            .delete_category(CategoryId::seed(1))
        });
        assert_eq!(refusal, Err(ReassignRefusal::HasChildren));
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
}
