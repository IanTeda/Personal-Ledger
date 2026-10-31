//! The Categories writes that cross into other domains (ADR-0038): each a command value with its
//! own refusal, checked before anything is written so a refusal leaves the Ledger untouched, and
//! recording an upsert or delete per row it wrote (ADR-0039).
//!
//! Add, rename, move and the type change are `lib_categories`' Interactors (#859). What stays here
//! is the Ledger's until its domains persist: the Monthly budget a Category Dialog saves writes a
//! Budget (#811), and a delete re-points Splits and Bill Plans, clears Payee defaults and drops
//! Budget Limits, the Desktop's answer to `lib_categories::CategoryReassign` until
//! `lib-reassign` (#813).

use chrono::NaiveDate;
use lib_budgets::BudgetError;
use lib_categories::{get, get_or_create_uncategorised, is_leaf};
use lib_core::{BudgetId, CategoryId, Money};

use crate::{Ledger, LedgerChange, LedgerCommand};

/// The Monthly budget a Category dialog saves into one Budget: an Onward amount from the current
/// month, or a Stop when `limit` is `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthlyBudget {
    pub budget_id: BudgetId,
    pub today: NaiveDate,
    pub limit: Option<Money>,
}

/// Why a Category delete was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DeleteCategoryRefusal {
    #[error("category not found")]
    NotFound,
    /// Only leaves are deleted; a parent's children go first.
    #[error("delete or move its children first")]
    HasChildren,
    /// Uncategorised is where a deleted Category's rows go, so it can't go while anything uses it.
    #[error("Uncategorised can't be deleted while anything uses it")]
    UncategorisedInUse,
}

/// What a Category delete did, for the Toast.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryDeleted {
    pub name: String,
    pub splits_moved: usize,
    pub bill_plans_moved: usize,
    pub payee_defaults_cleared: usize,
    pub budgets_touched: usize,
}

impl Ledger {
    /// Saves Category `id`'s Monthly budget; an unchanged value writes nothing. Returns whether it
    /// wrote.
    pub fn set_category_monthly_limit(
        &mut self,
        id: CategoryId,
        monthly: &MonthlyBudget,
    ) -> Result<bool, BudgetError> {
        self.write_budgets(|budgets, reads| {
            budgets.set_monthly_limit(
                monthly.budget_id,
                reads,
                id,
                monthly.limit.clone(),
                monthly.today,
            )
        })
    }

    /// Deletes leaf Category `id`. Its Splits and Bill Plans move to the Uncategorised Category of
    /// its type (created only if something moves), Payees that default to it lose the default,
    /// and every Budget drops its Category Limits.
    pub fn delete_category(
        &mut self,
        id: CategoryId,
    ) -> Result<CategoryDeleted, DeleteCategoryRefusal> {
        let category = get(&self.categories, id)
            .ok_or(DeleteCategoryRefusal::NotFound)?
            .clone();
        if !is_leaf(&self.categories, id) {
            return Err(DeleteCategoryRefusal::HasChildren);
        }
        let splits_using = self
            .transactions
            .iter()
            .flat_map(|t| &t.splits)
            .filter(|s| s.category_id == id)
            .count();
        let plans_using = self
            .bill_plans
            .iter()
            .filter(|p| p.category_id == id)
            .count();
        let needs_home = splits_using + plans_using > 0;
        // Re-pointing Uncategorised's rows onto itself would leave them dangling.
        let is_uncategorised = category.name == "Uncategorised" && category.parent.is_none();
        if is_uncategorised && needs_home {
            return Err(DeleteCategoryRefusal::UncategorisedInUse);
        }

        // Nothing can refuse past here.
        let mut changes = Vec::new();
        let home = needs_home.then(|| {
            let before = self.categories.len();
            let home =
                get_or_create_uncategorised(&mut self.categories, category.category_type.clone());
            if self.categories.len() > before
                && let Some(created) = get(&self.categories, home)
            {
                changes.push(LedgerChange::UpsertCategory(created.clone()));
            }
            home
        });

        let mut splits_moved = 0;
        let mut bill_plans_moved = 0;
        if let Some(home) = home {
            for transaction in &mut self.transactions {
                let mut touched = false;
                for split in transaction
                    .splits
                    .iter_mut()
                    .filter(|s| s.category_id == id)
                {
                    split.category_id = home;
                    splits_moved += 1;
                    touched = true;
                }
                if touched {
                    changes.push(LedgerChange::UpsertTransaction(transaction.clone()));
                }
            }
            for plan in self.bill_plans.iter_mut().filter(|p| p.category_id == id) {
                plan.category_id = home;
                bill_plans_moved += 1;
                changes.push(LedgerChange::UpsertBillPlan(plan.clone()));
            }
        }

        let mut payee_defaults_cleared = 0;
        for payee in self
            .payees
            .iter_mut()
            .filter(|p| p.default_category == Some(id))
        {
            payee.default_category = None;
            payee_defaults_cleared += 1;
            changes.push(LedgerChange::UpsertPayee(payee.clone()));
        }

        // `Budgets::remove_category` doesn't say which Budgets it touched, so list them first.
        let budgeted: Vec<BudgetId> = self
            .budgets
            .all()
            .iter()
            .filter(|b| b.limits.contains_key(&id))
            .map(|b| b.id)
            .collect();
        self.budgets.remove_category(id);
        for budget in budgeted.iter().filter_map(|id| self.budgets.get(*id)) {
            changes.push(LedgerChange::UpsertBudget(budget.clone()));
        }

        self.categories.retain(|c| c.id != id);
        changes.push(LedgerChange::DeleteCategory(id));
        self.pending.extend(changes);

        Ok(CategoryDeleted {
            name: category.name,
            splits_moved,
            bill_plans_moved,
            payee_defaults_cleared,
            budgets_touched: budgeted.len(),
        })
    }
}

/// [`Ledger::set_category_monthly_limit`] as a value: the Monthly budget a Category Dialog
/// saves, after the Category itself is written and the Ledger's read copy brought in line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetCategoryMonthlyLimit {
    pub id: CategoryId,
    pub monthly: MonthlyBudget,
}

impl LedgerCommand for SetCategoryMonthlyLimit {
    type Outcome = bool;
    type Refusal = BudgetError;
    fn run(self, ledger: &mut Ledger) -> Result<bool, BudgetError> {
        ledger.set_category_monthly_limit(self.id, &self.monthly)
    }
}

/// [`Ledger::delete_category`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteCategory(pub CategoryId);

impl LedgerCommand for DeleteCategory {
    type Outcome = CategoryDeleted;
    type Refusal = DeleteCategoryRefusal;
    fn run(self, ledger: &mut Ledger) -> Result<CategoryDeleted, DeleteCategoryRefusal> {
        ledger.delete_category(self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lib_budgets::PERSONAL_SPENDING_ID;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 10, 11).expect("a real date")
    }

    fn ledger() -> Ledger {
        Ledger::stub_on(today())
    }

    fn category_id(ledger: &Ledger, name: &str) -> CategoryId {
        lib_categories::find_by_name(ledger.categories(), name).expect("a seeded Category")
    }

    fn snapshot(ledger: &Ledger) -> String {
        format!("{ledger:?}")
    }

    fn money(text: &str) -> Money {
        text.parse().expect("a money amount")
    }

    fn monthly(limit: Option<Money>) -> MonthlyBudget {
        MonthlyBudget {
            budget_id: PERSONAL_SPENDING_ID,
            today: today(),
            limit,
        }
    }

    #[test]
    fn a_monthly_limit_records_its_budget_and_an_unchanged_one_writes_nothing() {
        let mut ledger = ledger();
        let groceries = category_id(&ledger, "Groceries");
        let set = SetCategoryMonthlyLimit {
            id: groceries,
            monthly: monthly(Some(money("612"))),
        };

        assert_eq!(set.clone().run(&mut ledger), Ok(true));
        assert!(
            ledger.drain_changes().iter().any(
                |c| matches!(c, LedgerChange::UpsertBudget(b) if b.id == PERSONAL_SPENDING_ID)
            )
        );
        assert_eq!(set.run(&mut ledger), Ok(false));
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn a_monthly_limit_on_a_missing_budget_is_refused_untouched() {
        let mut ledger = ledger();
        let before = snapshot(&ledger);
        let refusal = SetCategoryMonthlyLimit {
            id: category_id(&ledger, "Groceries"),
            monthly: MonthlyBudget {
                budget_id: BudgetId::new(),
                ..monthly(Some(money("1")))
            },
        }
        .run(&mut ledger);

        assert_eq!(refusal, Err(BudgetError::NotFound));
        assert_eq!(snapshot(&ledger), before);
    }

    #[test]
    fn deleting_a_missing_or_parent_category_is_refused_untouched() {
        let mut ledger = ledger();
        let food = category_id(&ledger, "Food");
        let before = snapshot(&ledger);

        assert_eq!(
            ledger.delete_category(CategoryId::seed(u32::MAX)),
            Err(DeleteCategoryRefusal::NotFound)
        );
        assert_eq!(
            ledger.delete_category(food),
            Err(DeleteCategoryRefusal::HasChildren)
        );
        assert_eq!(snapshot(&ledger), before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn deleting_a_used_category_moves_its_rows_and_records_each_change() {
        let mut ledger = ledger();
        let groceries = category_id(&ledger, "Groceries");
        let splits_before = ledger
            .transactions()
            .iter()
            .flat_map(|t| &t.splits)
            .filter(|s| s.category_id == groceries)
            .count();
        assert!(splits_before > 0, "the stub books Groceries");

        let deleted = ledger.delete_category(groceries).expect("a leaf");

        assert_eq!(deleted.name, "Groceries");
        assert_eq!(deleted.splits_moved, splits_before);
        assert!(get(ledger.categories(), groceries).is_none());
        let home = category_id(&ledger, "Uncategorised");
        assert!(
            ledger
                .transactions()
                .iter()
                .flat_map(|t| &t.splits)
                .all(|s| s.category_id != groceries)
        );
        let changes = ledger.drain_changes();
        assert!(changes.contains(&LedgerChange::DeleteCategory(groceries)));
        assert!(
            changes
                .iter()
                .any(|c| matches!(c, LedgerChange::UpsertCategory(c) if c.id == home))
        );
        assert!(
            changes
                .iter()
                .any(|c| matches!(c, LedgerChange::UpsertTransaction(_)))
        );
    }

    #[test]
    fn deleting_an_unused_category_creates_no_uncategorised() {
        let mut ledger = ledger();
        let food = category_id(&ledger, "Food");
        let id = CategoryId::new();
        let mut categories = ledger.categories().to_vec();
        categories.push(lib_categories::Category {
            id,
            name: "Snacks".into(),
            parent: Some(food),
            category_type: lib_core::CategoryTypes::Expense,
        });
        ledger.refresh_categories(categories);
        ledger.drain_changes();

        let deleted = ledger.delete_category(id).expect("a leaf");

        assert_eq!(deleted.splits_moved + deleted.bill_plans_moved, 0);
        assert!(lib_categories::find_by_name(ledger.categories(), "Uncategorised").is_none());
        assert_eq!(
            ledger.drain_changes(),
            vec![LedgerChange::DeleteCategory(id)]
        );
    }

    #[test]
    fn deleting_clears_payee_defaults_and_budget_limits() {
        let mut ledger = ledger();
        let groceries = category_id(&ledger, "Groceries");
        let defaults = ledger
            .payees()
            .iter()
            .filter(|p| p.default_category == Some(groceries))
            .count();
        let budgeted = ledger
            .budgets()
            .all()
            .iter()
            .filter(|b| b.limits.contains_key(&groceries))
            .count();

        let deleted = ledger.delete_category(groceries).expect("a leaf");

        assert_eq!(deleted.payee_defaults_cleared, defaults);
        assert_eq!(deleted.budgets_touched, budgeted);
        assert!(
            ledger
                .payees()
                .iter()
                .all(|p| p.default_category != Some(groceries))
        );
        assert!(
            ledger
                .budgets()
                .all()
                .iter()
                .all(|b| !b.limits.contains_key(&groceries))
        );
    }

    #[test]
    fn uncategorised_can_not_be_deleted_while_anything_uses_it() {
        let mut ledger = ledger();
        let groceries = category_id(&ledger, "Groceries");
        ledger.delete_category(groceries).expect("a leaf");
        ledger.drain_changes();
        let home = category_id(&ledger, "Uncategorised");
        let before = snapshot(&ledger);

        assert_eq!(
            ledger.delete_category(home),
            Err(DeleteCategoryRefusal::UncategorisedInUse)
        );
        assert_eq!(snapshot(&ledger), before);
    }
}
