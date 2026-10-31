//! The Category tree as Budgets walks it, over the [`BudgetCategory`] rows its port returns in
//! tree order.

use lib_core::{CategoryId, CategoryTypes};

use super::protocols::BudgetCategory;

pub(super) fn get(categories: &[BudgetCategory], id: CategoryId) -> Option<&BudgetCategory> {
    categories.iter().find(|category| category.id == id)
}

pub(super) fn find_by_name(categories: &[BudgetCategory], name: &str) -> Option<CategoryId> {
    categories
        .iter()
        .find(|category| category.name == name)
        .map(|category| category.id)
}

pub(super) fn is_expense(categories: &[BudgetCategory], id: CategoryId) -> bool {
    get(categories, id).is_some_and(|category| category.category_type == CategoryTypes::Expense)
}

/// Whether nothing is nested under `id`; an id the tree lacks has nothing under it.
pub(super) fn is_leaf(categories: &[BudgetCategory], id: CategoryId) -> bool {
    get(categories, id).is_none_or(|category| category.is_leaf)
}

/// `id` and every Category nested under it, at any depth.
pub(super) fn descendants_inclusive(
    categories: &[BudgetCategory],
    id: CategoryId,
) -> Vec<CategoryId> {
    let mut found = vec![id];
    let mut index = 0;
    while index < found.len() {
        let current = found[index];
        for category in categories {
            if category.parent == Some(current) && !found.contains(&category.id) {
                found.push(category.id);
            }
        }
        index += 1;
    }
    found
}
