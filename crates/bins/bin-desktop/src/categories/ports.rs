//! The Categories ports as the Desktop holds them (#858, #859): the store `shell::router::build` is
//! given, shared by the Categories module, Category delete (`crate::category_deletion`, #1005), the
//! Bills ports (`BillCategories`) and the Payees ports and the Import's
//! Category select (`PayeeCategories`, `ImportCategories`). The writes are committed by the input
//! they were made in (`crate::input::send`), not here.
//!
//! The Categories page writes the repository through `lib-categories`' Interactors. What the
//! module reaches of other domains goes through [`LedgerPorts`]: the usage its Dialogs quote (the
//! Splits from the Transactions store, #926, the Budgets from the Budgets store, #952), the
//! Monthly budget the Add and Edit Dialogs read and save in the Personal spending Budget, on the
//! Budgets store (#952), and the handover of a leaf's rows to its first child, written to the
//! Transactions, Bills and Payees stores ([`LedgerPorts`]'s `hand_over`). What names a Category,
//! the holdings, the handover and the type-change blockers' Bill Plans, is `lib-database`'s
//! `StoredCategoryRows`, the one selection Category delete reads too (#1034). The Open Ledger
//! hands the page these stores as [`CategoryStores`] (#1057). The Ledger keeps no read copy of the
//! Categories (#1055), and the input a write is made in announces it (#1081).

use std::cell::RefCell;
use std::rc::Rc;

use chrono::Datelike as _;
use lib_budgets::{Budgets, Error};
use lib_categories::{
    Category, CategoryBudget, CategoryHandover, CategoryRepository, CategoryUsage, CategoryUse,
    Holdings, MonthlyBudget, MonthlyBudgetRefusal, ParentStop, SharedCategories,
    TypeChangeBlockers,
};
use lib_core::{CategoryId, Clock, Money};
use lib_database::{SharedBudgetsStore, SharedCategoriesStore, StoredCategoryRows};

use crate::budgets::PERSONAL_SPENDING_ID;

/// The sample Categories in memory, for a test that needs no storage.
pub fn in_memory() -> SharedCategoriesStore {
    Rc::new(RefCell::new(lib_categories::fake::InMemoryCategories {
        categories: lib_categories::sample_categories(),
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

/// What the Categories ports reach beyond the page's own repository: the Categories store, the
/// Budgets store and the rows naming a Category in the Transactions, Bills and
/// Payees stores. The Open Ledger builds it once ([`crate::shell::router::LedgerStores`]).
#[derive(Clone)]
pub struct CategoryStores {
    pub categories: SharedCategoriesStore,
    pub budgets: SharedBudgetsStore,
    pub rows: StoredCategoryRows,
}

impl CategoryStores {
    /// The Categories store's tree as the Budgets rules read it.
    fn budget_categories(&self) -> Vec<lib_budgets::BudgetCategory> {
        use lib_budgets::BudgetCategories as _;
        lib_database::StoredBudgetCategories {
            categories: self.categories.clone(),
        }
        .budget_categories()
    }
}

/// The Categories ports the Desktop answers from its stores, built for one call: what names a
/// Category, the Monthly budget in the Personal spending Budget as of the day `clock` reads when
/// each call runs, and the handover. The input they are made in announces their writes.
pub struct LedgerPorts<'a> {
    pub stores: &'a CategoryStores,
    /// The Shell's Clock, read on each call so a Desktop left running past a month turn writes
    /// into the new month, never a closed one.
    pub clock: Clock,
}

impl LedgerPorts<'_> {
    /// The Budgets as the Budgets store holds them.
    fn budgets(&self) -> Budgets {
        Budgets::from_stored(self.stores.budgets.borrow().budgets().budgets())
    }
}

impl CategoryUsage for LedgerPorts<'_> {
    /// The Splits counted by the Transactions store (#926), and the Budgets holding a Category
    /// Limit for it by the Budgets store (#952).
    fn category_usage(&self, id: CategoryId) -> CategoryUse {
        CategoryUse {
            splits: self.stores.rows.splits(id),
            budgets: self.stores.budgets.borrow().category_budgets(id),
        }
    }

    /// The active Budgets using the subtree this month or later, and every Bill Plan naming it.
    fn type_change_blockers(&self, subtree: &[CategoryId]) -> TypeChangeBlockers {
        TypeChangeBlockers {
            budgets: self.budgets().using(subtree, self.clock.today()),
            bill_plans: self.stores.rows.bill_plans_naming(subtree),
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
                    &lib_database::budget_categories(categories),
                    id,
                    self.clock.today(),
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
                &lib_database::budget_categories(categories),
                category,
                amount.cloned(),
                self.clock.today(),
            )
            .map(|_| ())
            .map_err(budget_refusal)
    }

    /// Saves the amount to the Budgets store on the Categories store's tree, which holds the
    /// Category just written.
    fn set_monthly_budget(
        &mut self,
        category: CategoryId,
        amount: Option<Money>,
    ) -> Result<(), MonthlyBudgetRefusal> {
        let saved = crate::budgets::ports::set_monthly_limit(
            &mut *self.stores.budgets.borrow_mut(),
            &self.stores.budget_categories(),
            PERSONAL_SPENDING_ID,
            category,
            amount,
            self.clock.today(),
        );
        saved.map(|_| ()).map_err(budget_refusal)
    }

    fn parent_stop(&self, category: CategoryId) -> Option<ParentStop> {
        let today = self.clock.today();
        let budgets = self.budgets().budgeting(category, today);
        (budgets > 0).then_some(ParentStop {
            budgets,
            month: today.month(),
        })
    }

    /// Stops the parent in every active Budget in the Budgets store, in the input's one commit.
    fn stop_new_parent(&mut self, parent: CategoryId) {
        crate::budgets::ports::stop_new_parent(
            &mut *self.stores.budgets.borrow_mut(),
            parent,
            self.clock.today(),
        );
    }
}

impl CategoryHandover for LedgerPorts<'_> {
    fn holdings(&self, category: CategoryId) -> Holdings {
        self.stores.rows.holdings(category)
    }

    /// Re-points the rows in the Transactions, Bills and Payees stores, in the input's one commit.
    fn hand_over(&mut self, from: CategoryId, to: CategoryId) {
        self.stores.rows.clone().hand_over(from, to);
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use gpui::TestAppContext;

    use super::*;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 11).expect("a real date")
    }

    #[gpui::test]
    fn the_budget_port_checks_on_a_copy_and_saves_through_the_ledger(cx: &mut TestAppContext) {
        let stores = crate::shell::router::LedgerStores::sample(today()).category_stores();
        let categories = lib_categories::sample_categories();
        let groceries = lib_categories::find_by_name(&categories, "Groceries").expect("seeded");
        let salary = lib_categories::find_by_name(&categories, "Salary").expect("seeded");
        let amount: Money = "612".parse().expect("an amount");

        cx.update(|_| {
            let mut ports = LedgerPorts {
                stores: &stores,
                clock: Clock::fixed(today()),
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
    fn a_save_after_the_month_turns_starts_in_the_new_month(cx: &mut TestAppContext) {
        let october = today();
        let (clock, day) = Clock::settable(october);
        let stores = crate::shell::router::LedgerStores::sample(october).category_stores();
        let categories = lib_categories::sample_categories();
        let groceries = lib_categories::find_by_name(&categories, "Groceries").expect("seeded");
        let months = |stores: &CategoryStores| -> Vec<(i32, u32)> {
            stores
                .budgets
                .borrow()
                .budgets()
                .budget(PERSONAL_SPENDING_ID)
                .expect("the Personal spending Budget")
                .limits
                .get(&groceries)
                .map(|records| {
                    records
                        .iter()
                        .map(|record| (record.month.year, record.month.month))
                        .collect()
                })
                .unwrap_or_default()
        };
        let before = months(&stores);

        day.set(NaiveDate::from_ymd_opt(2026, 11, 2).expect("a real date"));
        cx.update(|_| {
            let mut ports = LedgerPorts {
                stores: &stores,
                clock,
            };
            ports
                .set_monthly_budget(groceries, Some("612".parse().expect("an amount")))
                .expect("a leaf Expense");
        });

        let after = months(&stores);
        assert!(
            after.contains(&(2026, 11)),
            "November starts a record: {after:?}"
        );
        assert_eq!(
            after.iter().filter(|&&month| month < (2026, 11)).count(),
            before.len(),
            "October's and earlier records are untouched: {after:?}"
        );
    }

    #[gpui::test]
    fn a_new_parent_is_stopped_in_every_active_budget_through_the_store(cx: &mut TestAppContext) {
        let stores = crate::shell::router::LedgerStores::sample(today()).category_stores();
        let categories = lib_categories::sample_categories();
        let groceries = lib_categories::find_by_name(&categories, "Groceries").expect("seeded");

        cx.update(|_| {
            let mut ports = LedgerPorts {
                stores: &stores,
                clock: Clock::fixed(today()),
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
        let stores = crate::shell::router::LedgerStores::sample(today()).category_stores();
        let categories = lib_categories::sample_categories();
        let groceries = lib_categories::find_by_name(&categories, "Groceries").expect("seeded");
        let child = lib_categories::find_by_name(&categories, "Dining")
            .expect("another seeded leaf stands in for a new child");

        cx.update(|_| {
            let mut ports = LedgerPorts {
                stores: &stores,
                clock: Clock::fixed(today()),
            };
            let held = ports.holdings(groceries);
            assert!(held.splits > 0, "the sample buys groceries");
            let child_before = ports.holdings(child);

            ports.hand_over(groceries, child);

            assert_eq!(ports.holdings(groceries), Holdings::default());
            assert_eq!(
                ports.holdings(child).splits,
                child_before.splits + held.splits
            );
        });
    }

    #[gpui::test]
    fn type_change_blockers_judge_the_budgets_by_the_clock_after_a_month_turn(
        cx: &mut TestAppContext,
    ) {
        let stores = crate::shell::router::LedgerStores::sample(today()).category_stores();
        let categories = lib_categories::sample_categories();
        let groceries = lib_categories::find_by_name(&categories, "Groceries").expect("seeded");
        let next_month = NaiveDate::from_ymd_opt(2026, 11, 1).expect("a real date");
        let (clock, day) = Clock::settable(today());

        cx.update(|_| {
            // Stop Groceries from next month in every active Budget.
            LedgerPorts {
                stores: &stores,
                clock: Clock::fixed(next_month),
            }
            .stop_new_parent(groceries);

            // Only the Clock moves; nothing is rebuilt.
            let ports = LedgerPorts {
                stores: &stores,
                clock,
            };
            assert!(ports.type_change_blockers(&[groceries]).budgets > 0);

            day.set(next_month);

            assert_eq!(ports.type_change_blockers(&[groceries]).budgets, 0);
        });
    }
}
