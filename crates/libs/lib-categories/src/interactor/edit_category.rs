use lib_core::{CategoryId, CategoryTypes};

use super::{BudgetChange, gains_first_child};
use crate::protocols::{CategoryRepository, CategorySavePorts};
use crate::{
    Category, CategoryError, SaveCategoryRefusal, change_category_type, edit_category,
    move_category,
};

/// The Edit category Dialog's save: rename Category `id` and move it under `parent_id` (top level
/// when `None`); at top level `category_type` cascades to every Category nested under it, and under
/// a parent the type follows the parent, so it is ignored there. Then its Monthly budget. Every
/// rule is checked over the rows before anything is written, so a refused move or budget leaves
/// the rename unwritten too.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditCategory {
    pub id: CategoryId,
    pub name: String,
    pub parent_id: Option<CategoryId>,
    pub category_type: Option<CategoryTypes>,
    pub monthly: BudgetChange,
}

impl EditCategory {
    /// Returns the Category as written.
    pub fn run(
        self,
        categories: &mut dyn CategoryRepository,
        budget: &mut dyn CategorySavePorts,
    ) -> Result<Category, SaveCategoryRefusal> {
        let before = categories.categories();
        let mut rows = before.clone();
        let outcome = self.edit(&mut rows).and_then(|category| {
            self.check_leaf_holds_nothing(&before, budget)?;
            self.check_type_change_unused(&before, &category, budget)?;
            self.monthly.check(budget, &rows, self.id)?;
            Ok(category)
        });
        match &outcome {
            Ok(category) => {
                // The Category and, after a type change, each Category nested under it.
                for row in rows.iter().filter(|row| !before.contains(row)) {
                    categories.upsert_category(row.clone());
                }
                self.monthly.save(budget, self.id);
                if let Some(parent) = gains_first_child(&before, self.parent_id) {
                    budget.stop_new_parent(parent);
                    budget.hand_over(parent, self.id);
                }
                tracing::info!(category_id = %category.id, "Edited a Category");
            }
            Err(refusal) => {
                tracing::info!(category_id = %self.id, %refusal, "Refused to edit a Category");
            }
        }
        outcome
    }

    /// A Category with children can't move under a leaf holding Splits, Bill Plans or Payee
    /// defaults, since only a first child takes them and a Category that already has children isn't
    /// one. A leaf moved there is that first child, so it is allowed.
    fn check_leaf_holds_nothing(
        &self,
        before: &[Category],
        ports: &dyn CategorySavePorts,
    ) -> Result<(), SaveCategoryRefusal> {
        let Some(target) = gains_first_child(before, self.parent_id) else {
            return Ok(());
        };
        if crate::is_leaf(before, self.id) {
            return Ok(());
        }
        let held = ports.holdings(target);
        if held.is_empty() {
            return Ok(());
        }
        let leaf = crate::get(before, target).map(|c| c.name.clone());
        Err(SaveCategoryRefusal::LeafHoldsRows {
            leaf: leaf.unwrap_or_default(),
            held,
        })
    }

    /// A top-level type change cascades to the subtree, so it is refused while a Budget or a Bill
    /// Plan uses any Category in it. `category` is the Category as edited.
    fn check_type_change_unused(
        &self,
        before: &[Category],
        category: &Category,
        ports: &dyn CategorySavePorts,
    ) -> Result<(), SaveCategoryRefusal> {
        let unchanged = crate::get(before, self.id)
            .is_none_or(|old| old.category_type == category.category_type);
        if unchanged {
            return Ok(());
        }
        let blockers = ports.type_change_blockers(&crate::descendants_inclusive(before, self.id));
        if blockers.is_empty() {
            return Ok(());
        }
        Err(SaveCategoryRefusal::TypeInUse {
            category: crate::get(before, self.id).map_or_else(String::new, |c| c.name.clone()),
            to: category.category_type.clone(),
            blockers,
        })
    }

    /// The rename, move and type change applied to `rows`, returning the Category as edited.
    fn edit(&self, rows: &mut [Category]) -> Result<Category, SaveCategoryRefusal> {
        edit_category(rows, self.id, self.name.clone())?;
        move_category(rows, self.id, self.parent_id)?;
        if let Some(category_type) = self
            .category_type
            .clone()
            .filter(|_| self.parent_id.is_none())
        {
            change_category_type(rows, self.id, category_type)?;
        }
        Ok(crate::get(rows, self.id)
            .cloned()
            .ok_or(CategoryError::NotFound)?)
    }
}

#[cfg(test)]
mod tests {
    use lib_core::Money;

    use super::*;
    use crate::MonthlyBudgetRefusal;
    use crate::fake::{InMemoryCategories, InMemoryCategoryBudget};

    const FOOD: CategoryId = CategoryId::seed(6);
    const GROCERIES: CategoryId = CategoryId::seed(7);
    const TRANSPORT: CategoryId = CategoryId::seed(9);
    const SALARY: CategoryId = CategoryId::seed(11);

    fn edit(id: CategoryId, name: &str, parent_id: Option<CategoryId>) -> EditCategory {
        EditCategory {
            id,
            name: name.into(),
            parent_id,
            category_type: None,
            monthly: BudgetChange::Keep,
        }
    }

    fn run(
        edit: EditCategory,
        categories: &mut InMemoryCategories,
    ) -> Result<Category, SaveCategoryRefusal> {
        edit.run(categories, &mut InMemoryCategoryBudget::default())
    }

    #[test]
    fn a_rename_and_move_land_together() {
        let mut categories = InMemoryCategories::stub();

        let edited = run(
            edit(TRANSPORT, "Getting around", Some(FOOD)),
            &mut categories,
        )
        .expect("Food takes children");

        assert_eq!(edited.name, "Getting around");
        assert_eq!(edited.parent, Some(FOOD));
        assert_eq!(categories.category(TRANSPORT), Some(edited));
    }

    #[test]
    fn a_top_level_type_change_cascades_to_its_subtree() {
        let mut categories = InMemoryCategories::stub();

        run(
            EditCategory {
                category_type: Some(CategoryTypes::Income),
                ..edit(FOOD, "Food", None)
            },
            &mut categories,
        )
        .expect("a top-level Category changes type");

        for id in [FOOD, GROCERIES, CategoryId::seed(8)] {
            assert_eq!(
                categories.category(id).map(|c| c.category_type),
                Some(CategoryTypes::Income)
            );
        }
        assert_eq!(
            categories.category(TRANSPORT).map(|c| c.category_type),
            Some(CategoryTypes::Expense)
        );
    }

    #[test]
    fn under_a_parent_the_type_asked_for_is_ignored() {
        let mut categories = InMemoryCategories::stub();

        let edited = run(
            EditCategory {
                category_type: Some(CategoryTypes::Income),
                ..edit(GROCERIES, "Groceries", Some(FOOD))
            },
            &mut categories,
        )
        .expect("an unchanged parent");

        assert_eq!(edited.category_type, CategoryTypes::Expense);
    }

    #[test]
    fn a_refused_move_leaves_the_rename_unwritten() {
        let mut categories = InMemoryCategories::stub();
        let before = categories.clone();

        assert_eq!(
            run(edit(FOOD, "Eating", Some(SALARY)), &mut categories),
            Err(CategoryError::TypeMismatch.into())
        );
        assert_eq!(
            run(edit(FOOD, "Eating", Some(GROCERIES)), &mut categories),
            Err(CategoryError::CycleDetected.into())
        );
        assert_eq!(
            run(
                edit(CategoryId::seed(999), "Nowhere", None),
                &mut categories
            ),
            Err(CategoryError::NotFound.into())
        );
        assert_eq!(categories, before);
    }

    #[test]
    fn a_refused_budget_leaves_the_edit_unwritten_and_a_stop_clears_it() {
        let mut categories = InMemoryCategories::stub();
        let before = categories.clone();
        let mut budget = InMemoryCategoryBudget::default();
        let forty: Money = "40".parse().expect("an amount");

        // Food is a parent, so it takes no amount of its own.
        assert_eq!(
            EditCategory {
                monthly: BudgetChange::Set(forty.clone()),
                ..edit(FOOD, "Eating", None)
            }
            .run(&mut categories, &mut budget),
            Err(MonthlyBudgetRefusal::NotAnExpenseLeaf.into())
        );
        assert_eq!(categories, before);

        budget.amounts.insert(GROCERIES, forty);
        EditCategory {
            monthly: BudgetChange::Stop,
            ..edit(GROCERIES, "Groceries", Some(FOOD))
        }
        .run(&mut categories, &mut budget)
        .expect("a leaf's budget stops");
        assert!(budget.amounts.is_empty());
    }

    #[test]
    fn moving_under_a_budgeted_leaf_stops_it_and_under_a_parent_stops_nothing() {
        let mut categories = InMemoryCategories::stub();
        let mut budget = InMemoryCategoryBudget::default();
        budget.amounts.insert(FOOD, "1".parse().expect("an amount"));

        // Food already has children: nothing to stop.
        edit(TRANSPORT, "Transport", Some(FOOD))
            .run(&mut categories, &mut budget)
            .expect("Food takes children");
        assert!(budget.stopped.is_empty());

        // Rent is a budgeted Expense leaf.
        let rent = CategoryId::seed(2);
        budget.amounts.insert(rent, "1".parse().expect("an amount"));
        edit(GROCERIES, "Groceries", Some(rent))
            .run(&mut categories, &mut budget)
            .expect("Rent takes a child");
        assert_eq!(budget.stopped, [rent]);
    }

    #[test]
    fn a_refused_move_stops_nothing() {
        let mut categories = InMemoryCategories::stub();
        let mut budget = InMemoryCategoryBudget::default();
        budget
            .amounts
            .insert(SALARY, "1".parse().expect("an amount"));

        assert!(
            edit(FOOD, "Food", Some(SALARY))
                .run(&mut categories, &mut budget)
                .is_err()
        );
        assert!(budget.stopped.is_empty());
    }

    #[test]
    fn a_parent_moved_under_a_leaf_with_rows_is_refused_and_writes_nothing() {
        let mut categories = InMemoryCategories::stub();
        let before = categories.clone();
        let mut budget = InMemoryCategoryBudget::default();
        budget.held.insert(TRANSPORT, held(0, 0, 1));
        budget
            .amounts
            .insert(GROCERIES, "1".parse().expect("an amount"));

        // Food has children; Transport is a leaf holding a Payee default.
        let refused = edit(FOOD, "Eating", Some(TRANSPORT)).run(&mut categories, &mut budget);

        assert_eq!(
            refused,
            Err(SaveCategoryRefusal::LeafHoldsRows {
                leaf: "Transport".into(),
                held: held(0, 0, 1),
            })
        );
        assert_eq!(categories, before, "the rename is unwritten too");
        assert!(budget.handed_over.is_empty());
        assert!(budget.stopped.is_empty());
    }

    #[test]
    fn a_parent_moved_under_an_empty_leaf_or_a_leaf_moved_under_one_with_rows_is_allowed() {
        let mut categories = InMemoryCategories::stub();
        let mut budget = InMemoryCategoryBudget::default();
        let household = CategoryId::seed(10);
        budget.held.insert(TRANSPORT, held(2, 0, 0));

        // Food has children, but Household holds nothing.
        edit(FOOD, "Food", Some(household))
            .run(&mut categories, &mut budget)
            .expect("an empty leaf takes a parent");
        // Interest is a leaf, so it is the first child that takes Salary's rows.
        budget.held.insert(SALARY, held(2, 0, 0));
        edit(CategoryId::seed(12), "Interest", Some(SALARY))
            .run(&mut categories, &mut budget)
            .expect("a leaf moved under a leaf with rows takes them");

        assert_eq!(budget.handed_over, [(SALARY, CategoryId::seed(12))]);
    }

    fn blocked(budgets: usize, bill_plans: usize) -> crate::TypeChangeBlockers {
        crate::TypeChangeBlockers {
            budgets,
            bill_plans,
        }
    }

    fn to_income(id: CategoryId, name: &str) -> EditCategory {
        EditCategory {
            category_type: Some(CategoryTypes::Income),
            ..edit(id, name, None)
        }
    }

    #[test]
    fn a_type_change_is_refused_for_a_used_leaf_in_the_subtree_and_writes_nothing() {
        for used in [blocked(1, 0), blocked(0, 1), blocked(2, 1)] {
            let mut categories = InMemoryCategories::stub();
            let before = categories.clone();
            let mut ports = InMemoryCategoryBudget::default();
            // Groceries is a leaf under Food.
            ports.blockers.insert(GROCERIES, used);

            let refused = to_income(FOOD, "Eating").run(&mut categories, &mut ports);

            assert_eq!(
                refused,
                Err(SaveCategoryRefusal::TypeInUse {
                    category: "Food".into(),
                    to: CategoryTypes::Income,
                    blockers: used,
                })
            );
            assert_eq!(categories, before, "the rename is unwritten too");
        }
    }

    #[test]
    fn a_type_change_is_allowed_for_an_unused_subtree_and_a_use_outside_it() {
        let mut categories = InMemoryCategories::stub();
        let mut ports = InMemoryCategoryBudget::default();
        // Nothing in Food's subtree is used (a chain Stopped before this month, or an archived
        // Budget, counts as no use): only Transport is.
        ports.blockers.insert(TRANSPORT, blocked(1, 1));

        to_income(FOOD, "Food")
            .run(&mut categories, &mut ports)
            .expect("an unused subtree changes type");

        assert_eq!(
            categories.category(GROCERIES).map(|c| c.category_type),
            Some(CategoryTypes::Income)
        );
    }

    #[test]
    fn an_unchanged_type_is_not_checked_against_usage() {
        let mut categories = InMemoryCategories::stub();
        let mut ports = InMemoryCategoryBudget::default();
        ports.blockers.insert(GROCERIES, blocked(1, 1));

        EditCategory {
            category_type: Some(CategoryTypes::Expense),
            ..edit(FOOD, "Eating", None)
        }
        .run(&mut categories, &mut ports)
        .expect("a rename keeps the type");
    }

    fn held(splits: usize, bill_plans: usize, payee_defaults: usize) -> crate::Holdings {
        crate::Holdings {
            splits,
            bill_plans,
            payee_defaults,
        }
    }

    #[test]
    fn moving_under_a_leaf_hands_its_rows_to_the_moved_category() {
        let mut categories = InMemoryCategories::stub();
        let mut budget = InMemoryCategoryBudget::default();
        budget.held.insert(SALARY, held(4, 1, 1));
        let moved = CategoryId::seed(12);

        edit(moved, "Moved", Some(SALARY))
            .run(&mut categories, &mut budget)
            .expect("Salary takes children");

        assert_eq!(budget.handed_over, [(SALARY, moved)]);
    }

    #[test]
    fn moving_under_an_existing_parent_or_a_refused_move_hands_nothing_over() {
        let mut categories = InMemoryCategories::stub();
        let mut budget = InMemoryCategoryBudget::default();
        budget.held.insert(FOOD, held(4, 0, 0));
        budget.held.insert(SALARY, held(4, 0, 0));

        edit(TRANSPORT, "Transport", Some(FOOD))
            .run(&mut categories, &mut budget)
            .expect("Food takes children");
        let refused = edit(TRANSPORT, "Transport", Some(SALARY)).run(&mut categories, &mut budget);

        assert!(refused.is_err(), "another type");
        assert!(budget.handed_over.is_empty());
    }
}
