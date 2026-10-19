//! The Categories writes (ADR-0038): each a command value with its own refusal, checked before
//! anything is written so a refusal leaves the Ledger untouched, and recording an upsert or delete
//! per row it wrote (ADR-0039).
//!
//! Single-Category rules (depth, type, cycles) stay in `lib_categories`. What lives here crosses
//! domains: the Monthly budget a Category dialog saves writes a Budget, and a delete re-points
//! Splits and Bill Plans, clears Payee defaults and drops Budget Limits.

use chrono::NaiveDate;
use lib_budgets::BudgetError;
use lib_categories::{
    Category, CategoryError, change_category_type, edit_category, get, get_or_create_uncategorised,
    insert_category, is_leaf, move_category,
};
use lib_core::{BudgetId, CategoryId, CategoryTypes, Money};

use crate::{Ledger, LedgerChange, LedgerCommand};

/// The Monthly budget a Category dialog saves into one Budget: an Onward amount from the current
/// month, or a Stop when `limit` is `None`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthlyBudget {
    pub budget_id: BudgetId,
    pub today: NaiveDate,
    pub limit: Option<Money>,
}

/// Why an Add or Edit of a Category was refused: the tree rules, or the Monthly budget it carried.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SaveCategoryRefusal {
    #[error(transparent)]
    Category(#[from] CategoryError),
    #[error(transparent)]
    Budget(#[from] BudgetError),
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
    /// Adds a Category and returns its id. A Monthly budget saves with it, and if the Budget
    /// refuses it the Category isn't added.
    pub fn add_category(
        &mut self,
        name: String,
        parent_id: Option<CategoryId>,
        category_type: CategoryTypes,
        monthly: Option<&MonthlyBudget>,
    ) -> Result<CategoryId, SaveCategoryRefusal> {
        self.atomic(|ledger| {
            let id = insert_category(&mut ledger.categories, name, parent_id, category_type)?;
            ledger.record_category(id);
            if let Some(monthly) = monthly {
                ledger.set_category_monthly_limit(id, monthly)?;
            }
            Ok(id)
        })
    }

    pub fn rename_category(&mut self, id: CategoryId, name: String) -> Result<(), CategoryError> {
        edit_category(&mut self.categories, id, name)?;
        self.record_category(id);
        Ok(())
    }

    /// Moves Category `id` under `parent_id` (`None` for top level).
    pub fn move_category(
        &mut self,
        id: CategoryId,
        parent_id: Option<CategoryId>,
    ) -> Result<(), CategoryError> {
        move_category(&mut self.categories, id, parent_id)?;
        self.record_category(id);
        Ok(())
    }

    /// Changes Category `id`'s type and its descendants', recording each row that changed.
    pub fn change_category_type(
        &mut self,
        id: CategoryId,
        category_type: CategoryTypes,
    ) -> Result<(), CategoryError> {
        let before = self.categories.clone();
        change_category_type(&mut self.categories, id, category_type)?;
        let changed: Vec<CategoryId> = self
            .categories
            .iter()
            .filter(|now| before.iter().find(|c| c.id == now.id) != Some(now))
            .map(|c| c.id)
            .collect();
        changed.into_iter().for_each(|id| self.record_category(id));
        Ok(())
    }

    /// Renames and moves Category `id` together, cascades `category_type` to its descendants when
    /// it ends at top level, then saves its Monthly budget, so the Edit dialog lands whole or not
    /// at all. Under a parent the type follows the parent, so `category_type` is ignored there.
    pub fn edit_category(
        &mut self,
        id: CategoryId,
        name: String,
        parent_id: Option<CategoryId>,
        category_type: Option<CategoryTypes>,
        monthly: Option<&MonthlyBudget>,
    ) -> Result<(), SaveCategoryRefusal> {
        self.atomic(|ledger| {
            ledger.rename_category(id, name)?;
            ledger.move_category(id, parent_id)?;
            if let Some(category_type) = category_type.filter(|_| parent_id.is_none()) {
                ledger.change_category_type(id, category_type)?;
            }
            if let Some(monthly) = monthly {
                ledger.set_category_monthly_limit(id, monthly)?;
            }
            Ok(())
        })
    }

    /// Saves Category `id`'s Monthly budget; an unchanged value writes nothing. Returns whether it
    /// wrote.
    pub fn set_category_monthly_limit(
        &mut self,
        id: CategoryId,
        monthly: &MonthlyBudget,
    ) -> Result<bool, BudgetError> {
        let wrote = self.budgets.set_monthly_limit(
            monthly.budget_id,
            &self.categories,
            id,
            monthly.limit.clone(),
            monthly.today,
        )?;
        if wrote && let Some(budget) = self.budgets.get(monthly.budget_id) {
            self.pending
                .push(LedgerChange::UpsertBudget(budget.clone()));
        }
        Ok(wrote)
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

    /// Runs a write that spans the tree and the Budgets, putting both and the recorded changes
    /// back if it refuses part-way.
    fn atomic<T, E>(&mut self, write: impl FnOnce(&mut Self) -> Result<T, E>) -> Result<T, E> {
        let categories = self.categories.clone();
        let budgets = self.budgets.clone();
        let recorded = self.pending.len();
        let result = write(self);
        if result.is_err() {
            self.categories = categories;
            self.budgets = budgets;
            self.pending.truncate(recorded);
        }
        result
    }

    fn record_category(&mut self, id: CategoryId) {
        if let Some(category) = get(&self.categories, id) {
            self.pending
                .push(LedgerChange::UpsertCategory(Category::clone(category)));
        }
    }
}

/// [`Ledger::add_category`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddCategory {
    pub name: String,
    pub parent_id: Option<CategoryId>,
    pub category_type: CategoryTypes,
    pub monthly: Option<MonthlyBudget>,
}

impl LedgerCommand for AddCategory {
    type Outcome = CategoryId;
    type Refusal = SaveCategoryRefusal;
    fn run(self, ledger: &mut Ledger) -> Result<CategoryId, SaveCategoryRefusal> {
        ledger.add_category(
            self.name,
            self.parent_id,
            self.category_type,
            self.monthly.as_ref(),
        )
    }
}

/// [`Ledger::rename_category`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameCategory {
    pub id: CategoryId,
    pub name: String,
}

impl LedgerCommand for RenameCategory {
    type Outcome = ();
    type Refusal = CategoryError;
    fn run(self, ledger: &mut Ledger) -> Result<(), CategoryError> {
        ledger.rename_category(self.id, self.name)
    }
}

/// [`Ledger::move_category`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveCategory {
    pub id: CategoryId,
    pub parent_id: Option<CategoryId>,
}

impl LedgerCommand for MoveCategory {
    type Outcome = ();
    type Refusal = CategoryError;
    fn run(self, ledger: &mut Ledger) -> Result<(), CategoryError> {
        ledger.move_category(self.id, self.parent_id)
    }
}

/// [`Ledger::change_category_type`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeCategoryType {
    pub id: CategoryId,
    pub category_type: CategoryTypes,
}

impl LedgerCommand for ChangeCategoryType {
    type Outcome = ();
    type Refusal = CategoryError;
    fn run(self, ledger: &mut Ledger) -> Result<(), CategoryError> {
        ledger.change_category_type(self.id, self.category_type)
    }
}

/// [`Ledger::edit_category`] as a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditCategory {
    pub id: CategoryId,
    pub name: String,
    pub parent_id: Option<CategoryId>,
    pub category_type: Option<CategoryTypes>,
    pub monthly: Option<MonthlyBudget>,
}

impl LedgerCommand for EditCategory {
    type Outcome = ();
    type Refusal = SaveCategoryRefusal;
    fn run(self, ledger: &mut Ledger) -> Result<(), SaveCategoryRefusal> {
        ledger.edit_category(
            self.id,
            self.name,
            self.parent_id,
            self.category_type,
            self.monthly.as_ref(),
        )
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
    fn adding_a_category_records_it_and_its_monthly_budget() {
        let mut ledger = ledger();
        let food = category_id(&ledger, "Food");
        let id = ledger
            .add_category(
                "Snacks".into(),
                Some(food),
                CategoryTypes::Expense,
                Some(&monthly(Some(money("50")))),
            )
            .expect("a leaf under an Expense parent");

        let changes = ledger.drain_changes();
        assert!(changes.contains(&LedgerChange::UpsertCategory(
            get(ledger.categories(), id).unwrap().clone()
        )));
        assert!(
            changes.iter().any(
                |c| matches!(c, LedgerChange::UpsertBudget(b) if b.id == PERSONAL_SPENDING_ID)
            )
        );
    }

    #[test]
    fn a_refused_add_leaves_the_ledger_untouched() {
        let mut ledger = ledger();
        let salary = category_id(&ledger, "Salary");
        let before = snapshot(&ledger);

        let refusal =
            ledger.add_category("Bonus".into(), Some(salary), CategoryTypes::Expense, None);

        assert_eq!(
            refusal,
            Err(SaveCategoryRefusal::Category(CategoryError::TypeMismatch))
        );
        assert_eq!(snapshot(&ledger), before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn a_refused_monthly_budget_takes_the_added_category_back() {
        let mut ledger = ledger();
        let before = snapshot(&ledger);
        let missing_budget = MonthlyBudget {
            budget_id: BudgetId::seed(u32::MAX),
            ..monthly(Some(money("5")))
        };

        let refusal = ledger.add_category(
            "Snacks".into(),
            None,
            CategoryTypes::Expense,
            Some(&missing_budget),
        );

        assert_eq!(
            refusal,
            Err(SaveCategoryRefusal::Budget(BudgetError::NotFound))
        );
        assert_eq!(snapshot(&ledger), before);
    }

    #[test]
    fn editing_renames_moves_and_records_the_category_once_per_step() {
        let mut ledger = ledger();
        let dining = category_id(&ledger, "Dining");
        ledger
            .edit_category(dining, "Eating out".into(), None, None, None)
            .expect("a move to top level");

        let category = get(ledger.categories(), dining).unwrap();
        assert_eq!(category.name, "Eating out");
        assert_eq!(category.parent, None);
        let changes = ledger.drain_changes();
        assert!(
            changes
                .iter()
                .all(|c| matches!(c, LedgerChange::UpsertCategory(c) if c.id == dining))
        );
        assert!(!changes.is_empty());
    }

    #[test]
    fn a_refused_move_takes_the_rename_back() {
        let mut ledger = ledger();
        let dining = category_id(&ledger, "Dining");
        let salary = category_id(&ledger, "Salary");
        let before = snapshot(&ledger);

        let refusal = ledger.edit_category(dining, "Eating out".into(), Some(salary), None, None);

        assert_eq!(
            refusal,
            Err(SaveCategoryRefusal::Category(CategoryError::TypeMismatch))
        );
        assert_eq!(snapshot(&ledger), before);
        assert!(ledger.drain_changes().is_empty());
    }

    fn recorded_categories(ledger: &mut Ledger) -> Vec<CategoryId> {
        let mut ids: Vec<CategoryId> = ledger
            .drain_changes()
            .into_iter()
            .filter_map(|c| match c {
                LedgerChange::UpsertCategory(c) => Some(c.id),
                _ => None,
            })
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    #[test]
    fn editing_a_top_level_type_cascades_to_its_descendants() {
        let mut ledger = ledger();
        let food = category_id(&ledger, "Food");
        ledger
            .edit_category(food, "Food".into(), None, Some(CategoryTypes::Income), None)
            .expect("a top-level Category");

        let mut expected = lib_categories::descendants_inclusive(ledger.categories(), food);
        expected.sort_unstable();
        assert!(expected.len() > 1, "the stub nests Categories under Food");
        assert!(expected.iter().all(|id| {
            get(ledger.categories(), *id).unwrap().category_type == CategoryTypes::Income
        }));
        assert_eq!(recorded_categories(&mut ledger), expected);
    }

    #[test]
    fn editing_without_a_type_change_touches_no_descendant() {
        let mut ledger = ledger();
        let food = category_id(&ledger, "Food");
        let types_before: Vec<CategoryTypes> = ledger
            .categories()
            .iter()
            .map(|c| c.category_type.clone())
            .collect();

        ledger
            .edit_category(
                food,
                "Groceries and dining".into(),
                None,
                Some(CategoryTypes::Expense),
                None,
            )
            .expect("an unchanged type");

        let types_after: Vec<CategoryTypes> = ledger
            .categories()
            .iter()
            .map(|c| c.category_type.clone())
            .collect();
        assert_eq!(types_after, types_before);
        assert_eq!(recorded_categories(&mut ledger), vec![food]);
    }

    #[test]
    fn a_type_for_a_category_with_a_parent_is_not_applied() {
        let mut ledger = ledger();
        let dining = category_id(&ledger, "Dining");
        let food = category_id(&ledger, "Food");

        ledger
            .edit_category(
                dining,
                "Dining".into(),
                Some(food),
                Some(CategoryTypes::Income),
                None,
            )
            .expect("the type follows the parent");

        assert_eq!(
            get(ledger.categories(), dining).unwrap().category_type,
            CategoryTypes::Expense
        );
    }

    #[test]
    fn a_refused_edit_takes_the_type_cascade_back() {
        let mut ledger = ledger();
        let food = category_id(&ledger, "Food");
        let before = snapshot(&ledger);
        let missing_budget = MonthlyBudget {
            budget_id: BudgetId::seed(u32::MAX),
            ..monthly(Some(money("5")))
        };

        let refusal = ledger.edit_category(
            food,
            "Food".into(),
            None,
            Some(CategoryTypes::Income),
            Some(&missing_budget),
        );

        assert_eq!(
            refusal,
            Err(SaveCategoryRefusal::Budget(BudgetError::NotFound))
        );
        assert_eq!(snapshot(&ledger), before);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn changing_a_type_records_every_descendant_that_changed() {
        let mut ledger = ledger();
        let food = category_id(&ledger, "Food");
        ledger
            .change_category_type(food, CategoryTypes::Income)
            .expect("a top-level Category");

        let changed: Vec<CategoryId> = ledger
            .drain_changes()
            .into_iter()
            .filter_map(|c| match c {
                LedgerChange::UpsertCategory(c) => Some(c.id),
                _ => None,
            })
            .collect();
        let mut expected = lib_categories::descendants_inclusive(ledger.categories(), food);
        expected.sort_unstable();
        let mut changed_sorted = changed;
        changed_sorted.sort_unstable();
        assert_eq!(changed_sorted, expected);
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
        let id = ledger
            .add_category("Snacks".into(), Some(food), CategoryTypes::Expense, None)
            .unwrap();
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
