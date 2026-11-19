//! Tree order (`GLOSSARY.md`): the one walk every crate lists Categories by, so `lib-categories`
//! and the feature crates' fakes, which can't depend on it, can't drift apart.

use std::collections::HashMap;

use crate::{CategoryId, CategoryRef, CategoryTypes};

/// A Category as Tree order reads it, whichever crate's row it is.
pub trait TreeNode {
    fn node_id(&self) -> CategoryId;
    fn node_parent(&self) -> Option<CategoryId>;
    fn node_is_income(&self) -> bool;
    fn node_name(&self) -> &str;
}

impl TreeNode for CategoryRef {
    fn node_id(&self) -> CategoryId {
        self.id
    }

    fn node_parent(&self) -> Option<CategoryId> {
        self.parent
    }

    fn node_is_income(&self) -> bool {
        self.category_type == CategoryTypes::Income
    }

    fn node_name(&self) -> &str {
        &self.name
    }
}

/// The indices of `rows` in Tree order: depth-first from each top-level row, each parent before
/// its children, siblings Expense before Income, then A-Z by name ignoring case, ties by id. A row
/// the walk can't reach (a missing parent or a cycle) is left out; the caller decides where it
/// goes. Each row's children are found once, so the walk is O(n log n) rather than a scan per row.
///
/// The walk starts only from rows with no parent, so a cycle among the other rows is never entered.
/// Each row is still placed at most once: two rows sharing an id (a corrupt Ledger) could otherwise
/// lead the walk back into a row it has already placed, and it would never end.
pub fn tree_order_indices<T: TreeNode>(rows: &[T]) -> Vec<usize> {
    let mut children: HashMap<Option<CategoryId>, Vec<usize>> = HashMap::new();
    for (index, row) in rows.iter().enumerate() {
        children.entry(row.node_parent()).or_default().push(index);
    }
    for siblings in children.values_mut() {
        siblings.sort_by_cached_key(|&index| {
            let row = &rows[index];
            (
                row.node_is_income(),
                row.node_name().to_lowercase(),
                row.node_id(),
            )
        });
    }

    let mut placed = vec![false; rows.len()];
    let mut out = Vec::with_capacity(rows.len());
    let mut stack: Vec<usize> = children
        .get(&None)
        .map(|top| top.iter().rev().copied().collect())
        .unwrap_or_default();
    while let Some(index) = stack.pop() {
        if std::mem::replace(&mut placed[index], true) {
            continue;
        }
        out.push(index);
        if let Some(under) = children.get(&Some(rows[index].node_id())) {
            stack.extend(under.iter().rev());
        }
    }
    out
}

/// `rows` in Tree order, leaving out any the walk can't reach, as [`tree_order_indices`] does.
pub fn tree_order<T: TreeNode + Clone>(rows: &[T]) -> Vec<T> {
    tree_order_indices(rows)
        .into_iter()
        .map(|index| rows[index].clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(n: u32, name: &str, parent: Option<u32>, category_type: CategoryTypes) -> CategoryRef {
        CategoryRef {
            id: CategoryId::seed(n),
            name: name.to_string(),
            path: name.to_string(),
            parent: parent.map(CategoryId::seed),
            category_type,
            is_leaf: false,
        }
    }

    fn names(rows: &[CategoryRef]) -> Vec<&str> {
        rows.iter().map(|row| row.name.as_str()).collect()
    }

    #[test]
    fn expense_comes_first_then_parents_before_children_and_siblings_a_to_z_ignoring_case() {
        let rows = vec![
            row(1, "salary", None, CategoryTypes::Income),
            row(2, "Food", None, CategoryTypes::Expense),
            row(3, "groceries", Some(2), CategoryTypes::Expense),
            row(4, "Dining", Some(2), CategoryTypes::Expense),
            row(5, "Bonus", None, CategoryTypes::Income),
        ];

        assert_eq!(
            names(&tree_order(&rows)),
            ["Food", "Dining", "groceries", "Bonus", "salary"]
        );
    }

    #[test]
    fn a_row_with_a_missing_parent_or_in_a_cycle_is_left_out() {
        let rows = vec![
            row(1, "Food", None, CategoryTypes::Expense),
            row(2, "Orphan", Some(9), CategoryTypes::Expense),
            row(3, "Loop", Some(3), CategoryTypes::Expense),
        ];

        assert_eq!(names(&tree_order(&rows)), ["Food"]);
    }

    #[test]
    fn a_cycle_below_a_top_level_row_is_left_out_and_the_walk_ends() {
        let rows = vec![
            row(1, "Food", None, CategoryTypes::Expense),
            row(2, "Ping", Some(3), CategoryTypes::Expense),
            row(3, "Pong", Some(2), CategoryTypes::Expense),
        ];

        assert_eq!(names(&tree_order(&rows)), ["Food"]);
    }

    #[test]
    fn two_rows_sharing_an_id_are_each_placed_once_and_the_walk_ends() {
        let rows = vec![
            row(1, "Food", None, CategoryTypes::Expense),
            row(1, "Again", Some(1), CategoryTypes::Expense),
        ];

        assert_eq!(names(&tree_order(&rows)), ["Food", "Again"]);
    }
}
