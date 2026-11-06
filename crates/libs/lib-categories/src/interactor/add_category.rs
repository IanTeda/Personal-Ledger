use lib_core::{CategoryId, CategoryTypes};

use super::{BudgetChange, gains_first_child};
use crate::protocols::{CategoryRepository, CategorySavePorts};
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
        budget: &mut dyn CategorySavePorts,
    ) -> Result<Category, SaveCategoryRefusal> {
        let before = categories.categories();
        let mut rows = before.clone();
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
                if let Some(parent) = gains_first_child(&before, self.parent_id) {
                    budget.stop_new_parent(parent);
                    budget.hand_over(parent, category.id);
                }
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
        let mut categories = InMemoryCategories::sample();
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
        let mut categories = InMemoryCategories::sample();
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
        let mut categories = InMemoryCategories::sample();
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
        let mut categories = InMemoryCategories::sample();
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

    #[test]
    fn a_budgeted_leaf_that_gains_its_first_child_is_stopped() {
        let mut categories = InMemoryCategories::sample();
        let mut budget = InMemoryCategoryBudget::default();
        let transport = CategoryId::seed(9);
        budget.amounts.insert(transport, money("600"));

        add("Fuel", Some(transport), CategoryTypes::Expense)
            .run(&mut categories, &mut budget)
            .expect("Transport takes children");

        assert_eq!(budget.stopped, [transport]);
    }

    #[test]
    fn a_parent_that_already_has_children_and_a_top_level_add_stop_nothing() {
        let mut categories = InMemoryCategories::sample();
        let mut budget = InMemoryCategoryBudget::default();

        add(
            "Takeaway",
            Some(CategoryId::seed(6)),
            CategoryTypes::Expense,
        )
        .run(&mut categories, &mut budget)
        .expect("Food takes children");
        add("Pets", None, CategoryTypes::Expense)
            .run(&mut categories, &mut budget)
            .expect("top level");

        assert!(budget.stopped.is_empty());
    }

    #[test]
    fn a_refused_add_stops_nothing() {
        let mut categories = InMemoryCategories::sample();
        let mut budget = InMemoryCategoryBudget::default();
        let salary = CategoryId::seed(11);
        budget.amounts.insert(salary, money("1"));

        let refused = AddCategory {
            monthly: BudgetChange::Set(money("40")),
            ..add("Bonus", Some(salary), CategoryTypes::Income)
        }
        .run(&mut categories, &mut budget);

        assert_eq!(refused, Err(MonthlyBudgetRefusal::NotAnExpenseLeaf.into()));
        assert!(budget.stopped.is_empty());
    }

    fn held(splits: usize, bill_plans: usize, payee_defaults: usize) -> crate::Holdings {
        crate::Holdings {
            splits,
            bill_plans,
            payee_defaults,
        }
    }

    #[test]
    fn a_leaf_that_gains_its_first_child_hands_over_its_splits_bill_plans_and_payee_defaults() {
        let mut categories = InMemoryCategories::sample();
        let mut budget = InMemoryCategoryBudget::default();
        let transport = CategoryId::seed(9);
        budget.held.insert(transport, held(42, 1, 2));

        let added = add("Fuel", Some(transport), CategoryTypes::Expense)
            .run(&mut categories, &mut budget)
            .expect("Transport takes children");

        assert_eq!(budget.handed_over, [(transport, added.id)]);
        assert_eq!(budget.held.get(&added.id), Some(&held(42, 1, 2)));
        assert!(!budget.held.contains_key(&transport));
    }

    #[test]
    fn a_leaf_holding_nothing_and_a_parent_with_children_hand_over_nothing() {
        let mut categories = InMemoryCategories::sample();
        let mut budget = InMemoryCategoryBudget::default();
        budget.held.insert(CategoryId::seed(6), held(3, 0, 0));

        add("Fuel", Some(CategoryId::seed(9)), CategoryTypes::Expense)
            .run(&mut categories, &mut budget)
            .expect("Transport takes children");
        add(
            "Takeaway",
            Some(CategoryId::seed(6)),
            CategoryTypes::Expense,
        )
        .run(&mut categories, &mut budget)
        .expect("Food takes children");

        assert!(budget.handed_over.is_empty());
    }

    #[test]
    fn a_refused_add_hands_nothing_over() {
        let mut categories = InMemoryCategories::sample();
        let mut budget = InMemoryCategoryBudget::default();
        let salary = CategoryId::seed(11);
        budget.held.insert(salary, held(5, 0, 0));

        let refused =
            add("Bonus", Some(salary), CategoryTypes::Expense).run(&mut categories, &mut budget);

        assert_eq!(refused, Err(CategoryError::TypeMismatch.into()));
        assert!(budget.handed_over.is_empty());
    }
}
