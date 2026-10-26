use lib_core::{CategoryId, CategoryTypes};

use super::BudgetChange;
use crate::protocols::{CategoryBudget, CategoryRepository};
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
        budget: &mut dyn CategoryBudget,
    ) -> Result<Category, SaveCategoryRefusal> {
        let before = categories.categories();
        let mut rows = before.clone();
        let outcome = self.edit(&mut rows).and_then(|category| {
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
                tracing::info!(category_id = %category.id, "Edited a Category");
            }
            Err(refusal) => {
                tracing::info!(category_id = %self.id, %refusal, "Refused to edit a Category");
            }
        }
        outcome
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
}
