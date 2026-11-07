//! `lib-categories`' ports answered over the stores: the Categories from the Categories store, what
//! names a Category and the handover of a leaf's rows from [`StoredCategoryRows`], and the Monthly
//! budget in the Personal spending Budget from the Budgets store (#858, #859, #1083).
//!
//! The Monthly budget and the type-change blockers are read as of the day the Clock reads when each
//! call runs, so a Client left running past a month turn writes into the new month, never a
//! closed one (#783).

use chrono::Datelike as _;
use lib_budgets::{Budgets, Error, PERSONAL_SPENDING_ID};
use lib_categories::{
    Category, CategoryBudget, CategoryHandover, CategoryRepository, CategoryUsage, CategoryUse,
    Holdings, MonthlyBudget, MonthlyBudgetRefusal, ParentStop, TypeChangeBlockers,
};
use lib_core::{CategoryId, Clock, Money};

use crate::adapter::store::{SharedBudgetsStore, SharedCategoriesStore, StoredBudgetCategories};
use crate::{StoredCategoryRows, budget_categories};

/// [`lib_categories`]' repository, usage, budget and handover ports over the shared Categories and
/// Budgets stores and the rows naming a Category, each borrowed for one call. A write reaches the
/// store's recorder, so the commit names every domain it touched: a handover names the
/// Transactions, Bills and Payees too.
///
/// It is `Clone` so a Client can hold the repository the page shares and the save ports a Dialog
/// writes through as two values over the same stores, whose borrows never meet.
#[derive(Clone)]
pub struct StoredCategoriesPorts {
    categories: SharedCategoriesStore,
    budgets: SharedBudgetsStore,
    rows: StoredCategoryRows,
    clock: Clock,
}

impl StoredCategoriesPorts {
    pub fn new(
        categories: SharedCategoriesStore,
        budgets: SharedBudgetsStore,
        rows: StoredCategoryRows,
        clock: Clock,
    ) -> Self {
        Self {
            categories,
            budgets,
            rows,
            clock,
        }
    }

    /// The Budgets as the Budgets store holds them.
    fn stored_budgets(&self) -> Budgets {
        Budgets::from_stored(self.budgets.borrow().budgets().budgets())
    }

    /// The Categories store's tree as the Budgets rules read it.
    fn tree(&self) -> StoredBudgetCategories {
        StoredBudgetCategories {
            categories: self.categories.clone(),
        }
    }
}

impl CategoryRepository for StoredCategoriesPorts {
    fn categories(&self) -> Vec<Category> {
        self.categories.borrow().categories().categories()
    }

    fn upsert_category(&mut self, category: Category) {
        self.categories
            .borrow_mut()
            .categories_mut()
            .upsert_category(category);
    }
}

impl CategoryUsage for StoredCategoriesPorts {
    /// The Splits counted by the Transactions store (#926), and the Budgets holding a Category
    /// Limit for it by the Budgets store (#952).
    fn category_usage(&self, id: CategoryId) -> CategoryUse {
        CategoryUse {
            splits: self.rows.splits(id),
            budgets: self.budgets.borrow().category_budgets(id),
        }
    }

    /// The active Budgets using the subtree this month or later, and every Bill Plan naming it.
    fn type_change_blockers(&self, subtree: &[CategoryId]) -> TypeChangeBlockers {
        TypeChangeBlockers {
            budgets: self.stored_budgets().using(subtree, self.clock.today()),
            bill_plans: self.rows.bill_plans_naming(subtree),
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

impl CategoryBudget for StoredCategoriesPorts {
    fn monthly_budget(
        &self,
        categories: &[Category],
        category: Option<CategoryId>,
    ) -> MonthlyBudget {
        let budgets = self.stored_budgets();
        MonthlyBudget {
            amount: category.and_then(|id| {
                budgets.monthly_limit(
                    PERSONAL_SPENDING_ID,
                    &budget_categories(categories),
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
        let mut budgets = self.stored_budgets();
        budgets
            .set_monthly_limit(
                PERSONAL_SPENDING_ID,
                &budget_categories(categories),
                category,
                amount.cloned(),
                self.clock.today(),
            )
            .map(|_| ())
            .map_err(budget_refusal)
    }

    /// Saves the amount to the Budgets store on the Categories store's tree, which holds the
    /// Category just written, as `lib-budgets`' `SetMonthlyLimit` Interactor writes it.
    fn set_monthly_budget(
        &mut self,
        category: CategoryId,
        amount: Option<Money>,
    ) -> Result<(), MonthlyBudgetRefusal> {
        let tree = self.tree();
        let today = self.clock.today();
        let mut budgets = self.budgets.borrow_mut();
        lib_budgets::SetMonthlyLimit {
            id: PERSONAL_SPENDING_ID,
            category_id: category,
            amount,
            today,
        }
        .run(&tree, budgets.budgets_mut())
        .map(|_| ())
        .map_err(budget_refusal)
    }

    fn parent_stop(&self, category: CategoryId) -> Option<ParentStop> {
        let today = self.clock.today();
        let budgets = self.stored_budgets().budgeting(category, today);
        (budgets > 0).then_some(ParentStop {
            budgets,
            month: today.month(),
        })
    }

    /// Stops the parent in every active Budget in the Budgets store, in the input's one commit, as
    /// `lib-budgets`' `StopNewParent` Interactor writes it.
    fn stop_new_parent(&mut self, parent: CategoryId) {
        lib_budgets::StopNewParent {
            category_id: parent,
            today: self.clock.today(),
        }
        .run(self.budgets.borrow_mut().budgets_mut());
    }
}

impl CategoryHandover for StoredCategoriesPorts {
    fn holdings(&self, category: CategoryId) -> Holdings {
        self.rows.holdings(category)
    }

    /// Re-points the rows in the Transactions, Bills and Payees stores, in the input's one commit.
    fn hand_over(&mut self, from: CategoryId, to: CategoryId) {
        self.rows.clone().hand_over(from, to);
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use lib_categories::Holdings;

    use super::*;
    use crate::ports::test_support::{fakes, file};
    use crate::{Domain, LedgerPorts};

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).expect("a real date")
    }

    fn groceries() -> CategoryId {
        let categories = lib_categories::sample_categories();
        lib_categories::find_by_name(&categories, "Groceries").expect("seeded")
    }

    #[test]
    fn a_category_write_reports_the_categories_written_on_either() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        for ledger in [fakes(), file(dir.path())] {
            let LedgerPorts {
                categories, commit, ..
            } = ledger;
            let mut category = categories
                .borrow()
                .categories()
                .into_iter()
                .next()
                .expect("a seeded Category");
            category.name.push_str(" renamed");

            categories.borrow_mut().upsert_category(category);

            assert_eq!(commit.commit().wrote, [Domain::Categories]);
        }
    }

    #[test]
    fn a_monthly_budget_write_reports_the_budgets_written_on_either() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        for ledger in [fakes(), file(dir.path())] {
            let LedgerPorts {
                category_save,
                commit,
                ..
            } = ledger;
            let mut ports = category_save;

            ports
                .set_monthly_budget(groceries(), Some("612".parse().expect("an amount")))
                .expect("a leaf Expense");

            assert_eq!(commit.commit().wrote, [Domain::Budgets]);
        }
    }

    #[test]
    fn a_handover_reports_the_rows_it_re_pointed_on_either() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let child = {
            let categories = lib_categories::sample_categories();
            lib_categories::find_by_name(&categories, "Dining").expect("another seeded leaf")
        };
        for ledger in [fakes(), file(dir.path())] {
            let LedgerPorts {
                category_save,
                commit,
                ..
            } = ledger;
            let mut ports = category_save;
            assert!(
                ports.holdings(groceries()).splits > 0,
                "the sample buys groceries"
            );

            ports.hand_over(groceries(), child);

            let wrote = commit.commit().wrote;
            assert!(wrote.contains(&Domain::Transactions), "{wrote:?}");
            assert_eq!(ports.holdings(groceries()), Holdings::default());
        }
    }

    #[test]
    fn a_save_after_the_month_turns_starts_in_the_new_month() {
        let (clock, day) = Clock::settable(today());
        let ledger = LedgerPorts::new(
            crate::Stores::fake(crate::FakeStores::sample(today())).0,
            clock,
        );
        let mut ports = ledger.category_save.clone();
        let months = || -> Vec<(i32, u32)> {
            ledger
                .stores
                .budgets
                .borrow()
                .budgets()
                .budget(PERSONAL_SPENDING_ID)
                .expect("the Personal spending Budget")
                .limits
                .get(&groceries())
                .map(|records| {
                    records
                        .iter()
                        .map(|record| (record.month.year, record.month.month))
                        .collect()
                })
                .unwrap_or_default()
        };
        let before = months();

        day.set(NaiveDate::from_ymd_opt(2026, 10, 2).expect("a real date"));
        ports
            .set_monthly_budget(groceries(), Some("612".parse().expect("an amount")))
            .expect("a leaf Expense");

        let after = months();
        assert!(
            after.contains(&(2026, 10)),
            "October starts a record: {after:?}"
        );
        assert_eq!(
            after.iter().filter(|&&month| month < (2026, 10)).count(),
            before.len(),
            "earlier records are untouched: {after:?}"
        );
    }

    #[test]
    fn a_new_parent_is_stopped_in_every_active_budget() {
        let ledger = fakes();
        let mut ports = ledger.category_save.clone();
        let categories = lib_categories::sample_categories();
        assert!(
            ports.parent_stop(groceries()).is_some(),
            "budgeted in the seed"
        );

        ports.stop_new_parent(groceries());

        assert_eq!(ports.parent_stop(groceries()), None);
        assert_eq!(
            ports.monthly_budget(&categories, Some(groceries())).amount,
            None
        );
    }
}
