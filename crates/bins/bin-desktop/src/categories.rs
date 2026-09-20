//! Shared stub Categories -- a tree, seeded from the sample tree in `docs/ux/desktop/Categories/`
//! (Housing, Food, Transport, Household, Salary, Interest and their children: 12 categories, three
//! levels deep). `gpui`-free, and deliberately small: the Transactions map needs paths, leaves and
//! descendants; the future Categories map grows this module (budgets, spent rollups, editing).
//!
//! Only **leaf** categories are assigned to Splits (glossary); a parent exists to roll up and to
//! filter by. All data is stubbed and in-memory.

use lib_core::CategoryTypes;

/// Joins a category's ancestors in a path label: `Food › Groceries`.
pub const PATH_SEPARATOR: &str = " \u{203a} ";

#[derive(Debug, Clone, PartialEq)]
pub struct Category {
    pub id: u32,
    pub name: String,
    /// `None` for a top-level category.
    pub parent: Option<u32>,
    pub category_type: CategoryTypes,
}

fn category(id: u32, name: &str, parent: Option<u32>, category_type: CategoryTypes) -> Category {
    Category {
        id,
        name: name.to_string(),
        parent,
        category_type,
    }
}

/// The Categories bundle's sample tree, parents before their children so the vector's own order is
/// already tree order.
pub fn default_categories() -> Vec<Category> {
    use CategoryTypes::{Expense, Income};
    vec![
        category(1, "Housing", None, Expense),
        category(2, "Rent", Some(1), Expense),
        category(3, "Utilities", Some(1), Expense),
        category(4, "Electricity", Some(3), Expense),
        category(5, "Water", Some(3), Expense),
        category(6, "Food", None, Expense),
        category(7, "Groceries", Some(6), Expense),
        category(8, "Dining", Some(6), Expense),
        category(9, "Transport", None, Expense),
        category(10, "Household", None, Expense),
        category(11, "Salary", None, Income),
        category(12, "Interest", None, Income),
    ]
}

/// The id of the category named `name` (first match), if any.
pub fn find_by_name(categories: &[Category], name: &str) -> Option<u32> {
    categories
        .iter()
        .find(|category| category.name == name)
        .map(|category| category.id)
}

fn get(categories: &[Category], id: u32) -> Option<&Category> {
    categories.iter().find(|category| category.id == id)
}

/// `Food › Groceries`: the category's ancestors then its own name. `None` for an unknown id.
pub fn path(categories: &[Category], id: u32) -> Option<String> {
    let mut names = vec![get(categories, id)?.name.as_str()];
    let mut current = get(categories, id)?.parent;
    // Bounded by the category count so a malformed cycle can't loop forever.
    for _ in 0..categories.len() {
        let Some(parent_id) = current else { break };
        let parent = get(categories, parent_id)?;
        names.push(parent.name.as_str());
        current = parent.parent;
    }
    names.reverse();
    Some(names.join(PATH_SEPARATOR))
}

/// Whether nothing is nested under `id` -- only leaves take Splits.
pub fn is_leaf(categories: &[Category], id: u32) -> bool {
    !categories
        .iter()
        .any(|category| category.parent == Some(id))
}

/// `id` and every category nested under it, at any depth -- what a filter on a parent matches.
pub fn descendants_inclusive(categories: &[Category], id: u32) -> Vec<u32> {
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

/// Every category with its [`path`], depth-first from each top-level category in list order --
/// the Category filter's option list, parents included (choosing a parent matches its descendants).
pub fn paths_in_tree_order(categories: &[Category]) -> Vec<(u32, String)> {
    fn walk(categories: &[Category], parent: Option<u32>, out: &mut Vec<(u32, String)>) {
        for category in categories.iter().filter(|c| c.parent == parent) {
            if let Some(label) = path(categories, category.id) {
                out.push((category.id, label));
            }
            walk(categories, Some(category.id), out);
        }
    }
    let mut out = Vec::with_capacity(categories.len());
    walk(categories, None, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_seed_matches_the_categories_bundles_twelve_category_tree() {
        let categories = default_categories();
        assert_eq!(categories.len(), 12);
        let mut ids: Vec<_> = categories.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), 12, "ids are unique");
        for category in &categories {
            if let Some(parent) = category.parent {
                assert!(
                    get(&categories, parent).is_some(),
                    "{} has a parent",
                    category.name
                );
            }
        }
    }

    #[test]
    fn paths_join_ancestors_with_the_separator() {
        let categories = default_categories();
        let electricity = find_by_name(&categories, "Electricity").unwrap();
        assert_eq!(
            path(&categories, electricity).as_deref(),
            Some("Housing \u{203a} Utilities \u{203a} Electricity")
        );
        let groceries = find_by_name(&categories, "Groceries").unwrap();
        assert_eq!(
            path(&categories, groceries).as_deref(),
            Some("Food \u{203a} Groceries")
        );
        let transport = find_by_name(&categories, "Transport").unwrap();
        assert_eq!(path(&categories, transport).as_deref(), Some("Transport"));
        assert_eq!(path(&categories, 999), None);
    }

    #[test]
    fn leaves_are_categories_with_no_children() {
        let categories = default_categories();
        let leaf = |name: &str| is_leaf(&categories, find_by_name(&categories, name).unwrap());
        assert!(leaf("Groceries"));
        assert!(leaf("Electricity"));
        assert!(
            leaf("Transport"),
            "a top-level category with no children is a leaf"
        );
        assert!(!leaf("Food"));
        assert!(!leaf("Utilities"));
        assert!(!leaf("Housing"));
    }

    #[test]
    fn descendants_include_the_category_and_everything_nested_at_any_depth() {
        let categories = default_categories();
        let id = |name: &str| find_by_name(&categories, name).unwrap();
        let mut housing = descendants_inclusive(&categories, id("Housing"));
        housing.sort_unstable();
        let mut expected = vec![
            id("Housing"),
            id("Rent"),
            id("Utilities"),
            id("Electricity"),
            id("Water"),
        ];
        expected.sort_unstable();
        assert_eq!(housing, expected);
        assert_eq!(
            descendants_inclusive(&categories, id("Groceries")),
            vec![id("Groceries")]
        );
    }

    #[test]
    fn paths_in_tree_order_lists_parents_before_their_children() {
        let categories = default_categories();
        let labels: Vec<_> = paths_in_tree_order(&categories)
            .into_iter()
            .map(|(_, label)| label)
            .collect();
        assert_eq!(labels.len(), 12);
        assert_eq!(labels[0], "Housing");
        assert_eq!(labels[1], "Housing \u{203a} Rent");
        assert_eq!(labels[2], "Housing \u{203a} Utilities");
        assert_eq!(labels[3], "Housing \u{203a} Utilities \u{203a} Electricity");
        assert!(labels.contains(&"Salary".to_string()));
    }

    #[test]
    fn income_and_expense_types_are_set() {
        let categories = default_categories();
        let salary = find_by_name(&categories, "Salary").unwrap();
        assert_eq!(
            get(&categories, salary).unwrap().category_type,
            CategoryTypes::Income
        );
        let groceries = find_by_name(&categories, "Groceries").unwrap();
        assert_eq!(
            get(&categories, groceries).unwrap().category_type,
            CategoryTypes::Expense
        );
    }
}
