//! Shared stub Budgets -- monthly targets per Category, seeded from the sample values in `docs/ux/desktop/Categories/`.
//! Budgets are defined per Category and Unit; rollup to parents; over-budget only applies to Expenses.
//! All data is stubbed and in-memory.

use lib_core::Money;

/// A monthly budget: a target amount for a Category and Unit.
#[derive(Debug, Clone, PartialEq)]
pub struct Budget {
    pub id: u32,
    pub category_id: u32,
    pub unit_id: u32,
    pub monthly_amount: Money,
}

fn budget(id: u32, category_id: u32, unit_id: u32, monthly_amount: Money) -> Budget {
    Budget {
        id,
        category_id,
        unit_id,
        monthly_amount,
    }
}

/// Builds an exact `Money` from a signed count of cents: `-1850` is `-18.50`.
fn cents_money(cents: i64) -> Money {
    use bigdecimal::BigDecimal;
    Money(BigDecimal::new(cents.into(), 2))
}

/// The default budgets seeded from the 5a mockup figures.
/// All budgets are in the base Unit (unit_id 1, typically AUD).
pub fn default_budgets() -> Vec<Budget> {
    vec![
        budget(1, 1, 1, cents_money(150_000)),  // Housing: 1500.00
        budget(2, 2, 1, cents_money(100_000)),  // Rent: 1000.00 (child of Housing)
        budget(3, 3, 1, cents_money(50_000)),   // Utilities: 500.00 (child of Housing)
        budget(4, 6, 1, cents_money(80_000)),   // Food: 800.00
        budget(5, 7, 1, cents_money(50_000)),   // Groceries: 500.00 (child of Food)
        budget(6, 8, 1, cents_money(30_000)), // Dining: 300.00 (child of Food, OVER BUDGET in seed)
        budget(7, 9, 1, cents_money(60_000)), // Transport: 600.00
        budget(8, 10, 1, cents_money(40_000)), // Household: 400.00
        budget(9, 11, 1, cents_money(400_000)), // Salary: 4000.00 (Income)
        budget(10, 12, 1, cents_money(50_000)), // Interest: 500.00 (Income)
    ]
}

/// Find a budget for a specific category and unit.
pub fn find_by_category_and_unit(
    budgets: &[Budget],
    category_id: u32,
    unit_id: u32,
) -> Option<&Budget> {
    budgets
        .iter()
        .find(|b| b.category_id == category_id && b.unit_id == unit_id)
}

/// Calculate the rollup budget for a category (sum of its children if they exist, otherwise its own budget).
pub fn rollup_budget(
    budgets: &[Budget],
    categories: &[crate::categories::Category],
    category_id: u32,
    unit_id: u32,
) -> Option<Money> {
    use bigdecimal::BigDecimal;

    let children: Vec<_> = categories
        .iter()
        .filter(|c| c.parent == Some(category_id))
        .map(|c| c.id)
        .collect();

    if children.is_empty() {
        // Leaf category: return its own budget
        find_by_category_and_unit(budgets, category_id, unit_id).map(|b| b.monthly_amount.clone())
    } else {
        // Parent category: sum children's rollups
        let mut total = BigDecimal::from(0);
        for child_id in children {
            if let Some(Money(amount)) = rollup_budget(budgets, categories, child_id, unit_id) {
                total += amount;
            }
        }
        Some(Money(total))
    }
}

/// Whether spending (an expense amount, negative) exceeds the budget for this category.
/// Only applies to Expense categories; returns false for Income.
pub fn is_over_budget(spent: &Money, budget: &Money) -> bool {
    spent.0 < budget.0.clone() * -1 // spent is negative; over-budget when |spent| > budget
}

/// Create a new budget for a category and unit. Returns the new budget ID.
pub fn create_budget(
    budgets: &mut Vec<Budget>,
    category_id: u32,
    unit_id: u32,
    monthly_amount: Money,
) -> u32 {
    let next_id = budgets.iter().map(|b| b.id).max().unwrap_or(0) + 1;
    budgets.push(Budget {
        id: next_id,
        category_id,
        unit_id,
        monthly_amount,
    });
    next_id
}

/// Update an existing budget's monthly amount. Creates if not found.
pub fn upsert_budget(
    budgets: &mut Vec<Budget>,
    category_id: u32,
    unit_id: u32,
    monthly_amount: Money,
) {
    if let Some(budget) = budgets
        .iter_mut()
        .find(|b| b.category_id == category_id && b.unit_id == unit_id)
    {
        budget.monthly_amount = monthly_amount;
    } else {
        create_budget(budgets, category_id, unit_id, monthly_amount);
    }
}

/// Delete the budget for a specific category and unit, if it exists.
pub fn delete_budget(budgets: &mut Vec<Budget>, category_id: u32, unit_id: u32) {
    budgets.retain(|b| !(b.category_id == category_id && b.unit_id == unit_id));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_seed_has_budgets_for_the_categories() {
        let budgets = default_budgets();
        assert!(budgets.len() >= 5);
        assert!(budgets.iter().any(|b| b.category_id == 6)); // Food
        assert!(budgets.iter().any(|b| b.category_id == 8)); // Dining (over budget)
    }

    #[test]
    fn budgets_are_unique_per_category_and_unit() {
        let budgets = default_budgets();
        for i in 0..budgets.len() {
            for j in (i + 1)..budgets.len() {
                assert_ne!(
                    (budgets[i].category_id, budgets[i].unit_id),
                    (budgets[j].category_id, budgets[j].unit_id),
                    "duplicate budget for category {} unit {}",
                    budgets[i].category_id,
                    budgets[i].unit_id
                );
            }
        }
    }

    #[test]
    fn dining_budget_is_set() {
        use bigdecimal::BigDecimal;

        let budgets = default_budgets();
        let dining_budget = find_by_category_and_unit(&budgets, 8, 1); // Dining, base unit
        assert!(dining_budget.is_some());
        assert_eq!(
            dining_budget.unwrap().monthly_amount.0,
            BigDecimal::new(30_000.into(), 2)
        );
    }
}
