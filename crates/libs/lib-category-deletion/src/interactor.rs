//! Category delete, the module's Interactor: it re-points rows in four other domains before
//! deleting the Category, so it lives here rather than in `lib-categories`. The rules moved from
//! `lib-ledger`'s `category_commands.rs` unchanged (#1004), and every refusal is checked before
//! anything is written, so a refusal writes nothing.

use lib_categories::{get, get_or_create_uncategorised, is_leaf, is_uncategorised};
use lib_core::CategoryId;

use crate::error::{Error, Result};
use crate::protocols::{CategoryDeleted, CategoryDeletion, CategoryRemoval};

/// Deletes leaf Category `id`. Its Splits and Bill Plans move to the Uncategorised Category of its
/// type (made only if something moves), Payees that default to it lose the default, and every
/// Budget, archived or not, drops its Limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteCategory {
    pub id: CategoryId,
}

impl DeleteCategory {
    /// # Errors
    /// `NotFound` for a Category that is gone, `HasChildren` for a parent, and
    /// `UncategorisedInUse` for an Uncategorised whose Splits or Bill Plans would have nowhere to
    /// go.
    pub fn run(self, deletion: &mut dyn CategoryDeletion) -> Result<CategoryDeleted> {
        let outcome = self.apply(deletion);
        match &outcome {
            Ok(deleted) => tracing::info!(
                category_id = %self.id,
                splits_moved = deleted.splits_moved,
                bill_plans_moved = deleted.bill_plans_moved,
                payee_defaults_cleared = deleted.payee_defaults_cleared,
                budgets_touched = deleted.budgets_touched,
                "Deleted a Category"
            ),
            Err(refusal) => {
                tracing::info!(category_id = %self.id, %refusal, "Refused to delete a Category");
            }
        }
        outcome
    }

    fn apply(self, deletion: &mut dyn CategoryDeletion) -> Result<CategoryDeleted> {
        let id = self.id;
        let mut tree = deletion.categories();
        let category = get(&tree, id).cloned().ok_or(Error::NotFound)?;
        if !is_leaf(&tree, id) {
            return Err(Error::HasChildren);
        }
        let mut transactions = deletion.transactions_using(id);
        let mut bill_plans = deletion.bill_plans_using(id);
        let needs_home = !transactions.is_empty() || !bill_plans.is_empty();
        // Re-pointing Uncategorised's rows onto itself would leave them dangling.
        if is_uncategorised(&category) && needs_home {
            return Err(Error::UncategorisedInUse);
        }

        // Nothing can refuse past here.
        let mut made = None;
        let mut splits_moved = 0;
        if needs_home {
            let before = tree.len();
            let home = get_or_create_uncategorised(&mut tree, category.category_type.clone());
            if tree.len() > before {
                made = get(&tree, home).cloned();
            }
            for split in transactions
                .iter_mut()
                .flat_map(|transaction| &mut transaction.splits)
                .filter(|split| split.category_id == id)
            {
                split.category_id = home;
                splits_moved += 1;
            }
            for plan in &mut bill_plans {
                plan.category_id = home;
            }
        }

        let mut payees = deletion.payees_defaulting_to(id);
        for payee in &mut payees {
            payee.default_category = None;
        }
        let mut budgets = deletion.budgets_limiting(id);
        for budget in &mut budgets {
            budget.limits.remove(&id);
        }

        let deleted = CategoryDeleted {
            id,
            name: category.name,
            splits_moved,
            bill_plans_moved: bill_plans.len(),
            payee_defaults_cleared: payees.len(),
            budgets_touched: budgets.len(),
        };
        deletion.remove(CategoryRemoval {
            home: made,
            transactions,
            bill_plans,
            payees,
            budgets,
            deleted: id,
        });
        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use lib_categories::{Category, CategoryRepository as _};
    use lib_core::CategoryTypes;

    use super::*;
    use crate::fake::InMemoryCategoryDeletion;
    use crate::fake::world::{DINING, FOOD, GROCERIES, SALARY, UNCATEGORISED_INCOME, WATER, world};

    /// The Uncategorised of `category_type` held, if any.
    fn uncategorised(
        deletion: &InMemoryCategoryDeletion,
        category_type: CategoryTypes,
    ) -> Option<Category> {
        deletion
            .categories
            .categories()
            .into_iter()
            .find(|category| is_uncategorised(category) && category.category_type == category_type)
    }

    #[test]
    fn a_used_category_moves_its_rows_to_a_new_uncategorised_and_goes() {
        let mut deletion = world();
        let payees = deletion.payees_defaulting_to(GROCERIES).len();
        let budgets = deletion.budgets_limiting(GROCERIES).len();

        let deleted = DeleteCategory { id: GROCERIES }
            .run(&mut deletion)
            .expect("Groceries is a leaf");

        assert_eq!(
            deleted,
            CategoryDeleted {
                id: GROCERIES,
                name: "Groceries".into(),
                splits_moved: 2,
                bill_plans_moved: 1,
                payee_defaults_cleared: payees,
                budgets_touched: budgets,
            }
        );
        let home = uncategorised(&deletion, CategoryTypes::Expense).expect("one is made");
        assert_eq!(deletion.categories.category(GROCERIES), None);
        assert!(deletion.transactions_using(GROCERIES).is_empty());
        assert_eq!(deletion.transactions_using(home.id).len(), 2);
        assert_eq!(deletion.bill_plans_using(home.id).len(), 1);
        assert_eq!(
            deletion.transactions_using(DINING).len(),
            1,
            "a Split on another Category stays"
        );
        assert!(deletion.payees_defaulting_to(GROCERIES).is_empty());
        assert!(deletion.budgets_limiting(GROCERIES).is_empty());
    }

    #[test]
    fn an_existing_uncategorised_of_the_type_is_reused() {
        let mut deletion = world();
        let before = deletion.categories.categories.len();

        let deleted = DeleteCategory { id: SALARY }
            .run(&mut deletion)
            .expect("Salary is a leaf");

        assert_eq!(deleted.splits_moved, 1);
        assert_eq!(deletion.categories.categories.len(), before - 1);
        assert_eq!(deletion.transactions_using(UNCATEGORISED_INCOME).len(), 2);
    }

    #[test]
    fn an_unused_category_makes_no_uncategorised() {
        let mut deletion = world();

        let deleted = DeleteCategory { id: WATER }
            .run(&mut deletion)
            .expect("Water is a leaf");

        assert_eq!(deleted.splits_moved + deleted.bill_plans_moved, 0);
        assert_eq!(uncategorised(&deletion, CategoryTypes::Expense), None);
        assert_eq!(deletion.categories.category(WATER), None);
    }

    #[test]
    fn a_missing_category_a_parent_or_an_uncategorised_in_use_is_refused_untouched() {
        let mut deletion = world();
        let before = deletion.clone();

        assert_eq!(
            DeleteCategory {
                id: CategoryId::seed(999)
            }
            .run(&mut deletion),
            Err(Error::NotFound)
        );
        assert_eq!(
            DeleteCategory { id: FOOD }.run(&mut deletion),
            Err(Error::HasChildren)
        );
        assert_eq!(
            DeleteCategory {
                id: UNCATEGORISED_INCOME
            }
            .run(&mut deletion),
            Err(Error::UncategorisedInUse)
        );
        assert_eq!(deletion, before);
    }

    #[test]
    fn an_unused_uncategorised_can_go() {
        let mut deletion = world();
        deletion.transactions.transactions.retain(|transaction| {
            transaction
                .splits
                .iter()
                .all(|split| split.category_id != UNCATEGORISED_INCOME)
        });

        DeleteCategory {
            id: UNCATEGORISED_INCOME,
        }
        .run(&mut deletion)
        .expect("nothing uses it");

        assert_eq!(uncategorised(&deletion, CategoryTypes::Income), None);
    }
}
