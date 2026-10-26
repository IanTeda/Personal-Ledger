use lib_core::{CategoryId, CategoryTypes};

use super::BudgetChange;
use crate::protocols::{CategoryBudget, CategoryRepository};
use crate::{Category, CategoryError, SaveCategoryRefusal, get, insert_category};

/// Add a Category at top level or under `parent_id`, with its Monthly budget. Refused when the
/// parent isn't there, is another type, or already sits at the depth cap, or when the budget
/// wouldn't save; a refusal writes neither.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddCategory {
    pub name: String,
    pub parent_id: Option<CategoryId>,
    pub category_type: CategoryTypes,
    pub monthly: BudgetChange,
}

impl AddCategory {
    /// Returns the Category as written.
    pub fn run(
        self,
        categories: &mut dyn CategoryRepository,
        budget: &mut dyn CategoryBudget,
    ) -> Result<Category, SaveCategoryRefusal> {
        let mut rows = categories.categories();
        let outcome = insert_category(&mut rows, self.name, self.parent_id, self.category_type)
            .and_then(|id| get(&rows, id).cloned().ok_or(CategoryError::NotFound))
            .map_err(SaveCategoryRefusal::from)
            .and_then(|category| {
                self.monthly.check(budget, &rows, category.id)?;
                Ok(category)
            });
        match &outcome {
            Ok(category) => {
                categories.upsert_category(category.clone());
                self.monthly.save(budget, category.id);
                tracing::info!(category_id = %category.id, "Added a Category");
            }
            Err(refusal) => tracing::info!(%refusal, "Refused to add a Category"),
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use lib_core::Money;

    use super::*;
    use crate::MonthlyBudgetRefusal;
    use crate::fake::{InMemoryCategories, InMemoryCategoryBudget};

    fn add(name: &str, parent_id: Option<CategoryId>, category_type: CategoryTypes) -> AddCategory {
        AddCategory {
            name: name.into(),
            parent_id,
            category_type,
            monthly: BudgetChange::Keep,
        }
    }

    fn money(text: &str) -> Money {
        text.parse().expect("an amount")
    }

    #[test]
    fn adding_a_child_writes_it_under_its_parent() {
        let mut categories = InMemoryCategories::stub();
        let food = CategoryId::seed(6);

        let added = add("Takeaway", Some(food), CategoryTypes::Expense)
            .run(&mut categories, &mut InMemoryCategoryBudget::default())
            .expect("Food takes children");

        assert_eq!(added.name, "Takeaway");
        assert_eq!(added.parent, Some(food));
        assert_eq!(categories.category(added.id), Some(added));
    }

    #[test]
    fn a_child_of_another_type_or_past_the_depth_cap_is_refused_and_nothing_is_written() {
        let mut categories = InMemoryCategories::stub();
        let before = categories.clone();
        let mut budget = InMemoryCategoryBudget::default();
        let salary = CategoryId::seed(11);
        let electricity = CategoryId::seed(4);

        assert_eq!(
            add("Bonus", Some(salary), CategoryTypes::Expense).run(&mut categories, &mut budget),
            Err(CategoryError::TypeMismatch.into())
        );
        assert_eq!(
            add("Peak", Some(electricity), CategoryTypes::Expense)
                .run(&mut categories, &mut budget),
            Err(CategoryError::DepthExceeded.into())
        );
        assert_eq!(
            add(
                "Orphan",
                Some(CategoryId::seed(999)),
                CategoryTypes::Expense
            )
            .run(&mut categories, &mut budget),
            Err(CategoryError::ParentNotFound.into())
        );
        assert_eq!(categories, before);
    }

    #[test]
    fn a_monthly_budget_saves_with_the_category() {
        let mut categories = InMemoryCategories::stub();
        let mut budget = InMemoryCategoryBudget::default();

        let added = AddCategory {
            monthly: BudgetChange::Set(money("40")),
            ..add("Pets", None, CategoryTypes::Expense)
        }
        .run(&mut categories, &mut budget)
        .expect("a top-level Expense leaf takes a budget");

        assert_eq!(budget.amounts.get(&added.id), Some(&money("40")));
    }

    #[test]
    fn a_budget_the_budget_refuses_writes_no_category_either() {
        let mut categories = InMemoryCategories::stub();
        let before = categories.clone();
        let mut budget = InMemoryCategoryBudget::default();

        assert_eq!(
            AddCategory {
                monthly: BudgetChange::Set(money("40")),
                ..add("Bonus", None, CategoryTypes::Income)
            }
            .run(&mut categories, &mut budget),
            Err(MonthlyBudgetRefusal::NotAnExpenseLeaf.into())
        );
        assert_eq!(categories, before);
        assert!(budget.amounts.is_empty());
    }
}
