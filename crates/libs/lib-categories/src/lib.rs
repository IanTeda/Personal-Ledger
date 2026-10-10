//! Shared stub Categories -- a tree, seeded from the sample tree in `docs/ux/desktop-mockups/18-categories/`
//! (Housing, Food, Transport, Household, Salary, Interest and their children: 12 categories, three
//! levels deep). `gpui`-free, deliberately grows from the Transactions map's need for paths, leaves
//! and descendants to support budgets, spent rollups, tree navigation with expand/collapse, and editing.
//!
//! Only **leaf** categories are assigned to Splits (glossary); a parent exists to roll up and to
//! filter by. All data is stubbed and in-memory. Depth is capped at 3 levels.

use bigdecimal::BigDecimal;
use chrono::Datelike;
use lib_core::{CategoryTypes, Money};

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

pub fn get(categories: &[Category], id: u32) -> Option<&Category> {
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

/// The depth of a category in the tree (0 for top-level, capped at 3).
pub fn depth(categories: &[Category], id: u32) -> u32 {
    let mut d = 0;
    let mut current = get(categories, id).and_then(|c| c.parent);
    while let Some(parent_id) = current {
        d += 1;
        current = get(categories, parent_id).and_then(|c| c.parent);
        if d >= 3 {
            break;
        }
    }
    d
}

/// Month-to-date amount for a category (and its descendants if a parent), summed from Transactions.
/// Expenses are negative; Income is positive. Returns zero if no transactions in the month.
pub fn month_to_date_spent(
    categories: &[Category],
    transactions: &[lib_transactions::Transaction],
    _accounts: &[lib_accounts::Account],
    category_id: u32,
    today: chrono::NaiveDate,
) -> Money {
    let category_ids = descendants_inclusive(categories, category_id);
    // Stepping back `day0` days lands on the 1st without `with_day`'s Option.
    let month_start = today - chrono::Duration::days(i64::from(today.day0()));

    let mut total = BigDecimal::from(0);
    for transaction in transactions {
        if transaction.date < month_start {
            continue;
        }
        if transaction.date > today {
            continue;
        }
        for split in &transaction.splits {
            if category_ids.contains(&split.category_id) {
                total += split.amount.0.clone();
            }
        }
    }
    Money(total)
}

/// Rollup of month-to-date for a parent category (sum of its children).
pub fn rollup_month_to_date(
    categories: &[Category],
    transactions: &[lib_transactions::Transaction],
    accounts: &[lib_accounts::Account],
    category_id: u32,
    today: chrono::NaiveDate,
) -> Money {
    let children: Vec<_> = categories
        .iter()
        .filter(|c| c.parent == Some(category_id))
        .map(|c| c.id)
        .collect();

    if children.is_empty() {
        month_to_date_spent(categories, transactions, accounts, category_id, today)
    } else {
        let mut total = BigDecimal::from(0);
        for child_id in children {
            let Money(amount) =
                month_to_date_spent(categories, transactions, accounts, child_id, today);
            total += amount;
        }
        Money(total)
    }
}

/// Find or create an Uncategorised category for a type. Returns the id if found or created.
pub fn get_or_create_uncategorised(
    categories: &mut Vec<Category>,
    category_type: CategoryTypes,
) -> u32 {
    if let Some(cat) = categories.iter().find(|c| {
        c.name == "Uncategorised" && c.category_type == category_type && c.parent.is_none()
    }) {
        return cat.id;
    }

    // Find next available id
    let next_id = categories.iter().map(|c| c.id).max().unwrap_or(0) + 1;
    categories.push(Category {
        id: next_id,
        name: "Uncategorised".to_string(),
        parent: None,
        category_type,
    });
    next_id
}

/// Errors for category operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CategoryError {
    /// Attempted to nest a category deeper than the 3-level cap.
    #[error("categories nest at most three levels deep")]
    DepthExceeded,
    /// Attempted to make a category its own parent (directly or via a cycle).
    #[error("a category can't be its own parent")]
    CycleDetected,
    /// Attempted to move an Expense to Income (or vice versa) or parent has different type.
    #[error("the parent category is a different type")]
    TypeMismatch,
    /// Attempted to delete a non-leaf category with children.
    #[error("delete or move its children first")]
    NonLeafDeletion,
    /// Category not found.
    #[error("category not found")]
    NotFound,
    /// Parent category not found.
    #[error("parent category not found")]
    ParentNotFound,
}

/// Whether `child_id` is a descendant of `parent_id` (cycle detection).
fn is_descendant(categories: &[Category], parent_id: u32, child_id: u32) -> bool {
    if parent_id == child_id {
        return true;
    }
    let children: Vec<_> = categories
        .iter()
        .filter(|c| c.parent == Some(parent_id))
        .map(|c| c.id)
        .collect();
    children
        .iter()
        .any(|&id| is_descendant(categories, id, child_id))
}

/// Insert a new category with validation.
/// - Parent must exist (if not None)
/// - Parent and category must match types
/// - Depth must not exceed 3 levels
/// - No cycles
pub fn insert_category(
    categories: &mut Vec<Category>,
    name: String,
    parent_id: Option<u32>,
    category_type: CategoryTypes,
) -> Result<u32, CategoryError> {
    if let Some(parent_id) = parent_id {
        let parent = get(categories, parent_id).ok_or(CategoryError::ParentNotFound)?;
        // Type must match
        if parent.category_type != category_type {
            return Err(CategoryError::TypeMismatch);
        }
        // Depth check
        if depth(categories, parent_id) >= 2 {
            return Err(CategoryError::DepthExceeded);
        }
    }

    let next_id = categories.iter().map(|c| c.id).max().unwrap_or(0) + 1;
    categories.push(Category {
        id: next_id,
        name,
        parent: parent_id,
        category_type,
    });
    Ok(next_id)
}

/// Edit a category's name (parent changes use `move_category`).
pub fn edit_category(
    categories: &mut [Category],
    id: u32,
    name: String,
) -> Result<(), CategoryError> {
    let category = categories
        .iter_mut()
        .find(|c| c.id == id)
        .ok_or(CategoryError::NotFound)?;
    category.name = name;
    Ok(())
}

/// Move a category to a new parent.
/// - New parent must exist (if not None)
/// - Types must match
/// - Depth must not exceed 3 levels
/// - No cycles allowed
pub fn move_category(
    categories: &mut [Category],
    id: u32,
    new_parent_id: Option<u32>,
) -> Result<(), CategoryError> {
    let category = categories
        .iter()
        .find(|c| c.id == id)
        .ok_or(CategoryError::NotFound)?;
    let category_type = category.category_type.clone();

    if let Some(parent_id) = new_parent_id {
        let parent = get(categories, parent_id).ok_or(CategoryError::ParentNotFound)?;
        // Type must match
        if parent.category_type != category_type {
            return Err(CategoryError::TypeMismatch);
        }
        // Cycle check: new parent must not be a descendant of id
        if is_descendant(categories, id, parent_id) {
            return Err(CategoryError::CycleDetected);
        }
        // Depth check
        if depth(categories, parent_id) >= 2 {
            return Err(CategoryError::DepthExceeded);
        }
    }

    if let Some(cat) = categories.iter_mut().find(|c| c.id == id) {
        cat.parent = new_parent_id;
    }
    Ok(())
}

/// Change a category's type and cascade to all descendants.
pub fn change_category_type(
    categories: &mut [Category],
    id: u32,
    new_type: CategoryTypes,
) -> Result<(), CategoryError> {
    let _category = categories
        .iter()
        .find(|c| c.id == id)
        .ok_or(CategoryError::NotFound)?;

    let descendants = descendants_inclusive(categories, id);
    for category in categories.iter_mut() {
        if descendants.contains(&category.id) {
            category.category_type = new_type.clone();
        }
    }
    Ok(())
}

/// Delete a category (leaf only). Re-points its splits to Uncategorised.
pub fn delete_category(categories: &mut Vec<Category>, id: u32) -> Result<(), CategoryError> {
    let category = get(categories, id).ok_or(CategoryError::NotFound)?;
    let category_type = category.category_type.clone();

    // Only leaf categories can be deleted
    if !is_leaf(categories, id) {
        return Err(CategoryError::NonLeafDeletion);
    }

    // Get or create Uncategorised (future: re-point splits to this category)
    let _uncategorised_id = get_or_create_uncategorised(categories, category_type);

    // Remove the category
    categories.retain(|c| c.id != id);

    Ok(())
}

/// Information about a category in the tree view (used for rendering).
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub id: u32,
    pub name: String,
    pub depth: u32,
    pub is_leaf: bool,
    pub is_expanded: bool,
    pub has_children: bool,
}

/// Get tree view rows in display order, respecting expand/collapse state.
/// Only includes nodes and their descendants if the node is expanded.
pub fn tree_rows(categories: &[Category], expanded: &[u32]) -> Vec<TreeNode> {
    fn walk(
        categories: &[Category],
        expanded: &[u32],
        parent: Option<u32>,
        out: &mut Vec<TreeNode>,
    ) {
        for category in categories.iter().filter(|c| c.parent == parent) {
            let d = depth(categories, category.id);
            let is_leaf = is_leaf(categories, category.id);
            let has_children = !is_leaf;
            let is_expanded = expanded.contains(&category.id);

            out.push(TreeNode {
                id: category.id,
                name: category.name.clone(),
                depth: d,
                is_leaf,
                is_expanded,
                has_children,
            });

            if is_expanded {
                walk(categories, expanded, Some(category.id), out);
            }
        }
    }
    let mut out = Vec::new();
    walk(categories, expanded, None, &mut out);
    out
}

/// Toggle the expanded state of a category (only for non-leaf categories).
pub fn toggle_expanded(expanded: &mut Vec<u32>, id: u32, is_leaf: bool) {
    if is_leaf {
        return;
    }
    if expanded.contains(&id) {
        expanded.retain(|&i| i != id);
    } else {
        expanded.push(id);
    }
}

/// The Settings page's visible rows, top to bottom: the Expense tree, then the Income tree, each
/// in tree order and respecting `expanded`. `j`/`k` walk exactly this list.
pub fn settings_rows(categories: &[Category], expanded: &[u32]) -> Vec<TreeNode> {
    let rows = tree_rows(categories, expanded);
    let is_expense = |row: &TreeNode| {
        get(categories, row.id).is_some_and(|c| c.category_type == CategoryTypes::Expense)
    };
    let mut ordered: Vec<TreeNode> = rows.iter().filter(|r| is_expense(r)).cloned().collect();
    ordered.extend(rows.iter().filter(|r| !is_expense(r)).cloned());
    ordered
}

/// How many levels the tree runs to: a lone top-level Category is one level.
pub fn levels_deep(categories: &[Category]) -> u32 {
    categories
        .iter()
        .map(|c| depth(categories, c.id) + 1)
        .max()
        .unwrap_or(0)
}

/// How many Categories sit directly under `id`.
pub fn child_count(categories: &[Category], id: u32) -> usize {
    categories.iter().filter(|c| c.parent == Some(id)).count()
}

/// The Categories data behind the Desktop's Categories Entity: the tree, read through `categories`
/// and changed only through the rule-checked methods below, so the depth, type and cycle rules
/// stay in one place. Persistence is a later phase, so the tree is in memory.
pub struct CategoryService {
    categories: Vec<Category>,
}

impl CategoryService {
    /// A service over `categories`, as the Desktop seeds it from [`default_categories`].
    pub fn new(categories: Vec<Category>) -> Self {
        Self { categories }
    }

    pub fn categories(&self) -> &[Category] {
        &self.categories
    }

    /// Adds a Category under `parent_id` and returns its id; see [`insert_category`].
    pub fn insert(
        &mut self,
        name: String,
        parent_id: Option<u32>,
        category_type: CategoryTypes,
    ) -> Result<u32, CategoryError> {
        insert_category(&mut self.categories, name, parent_id, category_type)
    }

    /// Renames Category `id`; see [`edit_category`].
    pub fn edit(&mut self, id: u32, name: String) -> Result<(), CategoryError> {
        edit_category(&mut self.categories, id, name)
    }

    /// Moves Category `id` under `new_parent_id` (`None` for top level); see [`move_category`].
    pub fn move_to(&mut self, id: u32, new_parent_id: Option<u32>) -> Result<(), CategoryError> {
        move_category(&mut self.categories, id, new_parent_id)
    }

    /// Changes Category `id`'s type and its descendants'; see [`change_category_type`].
    pub fn change_type(&mut self, id: u32, new_type: CategoryTypes) -> Result<(), CategoryError> {
        change_category_type(&mut self.categories, id, new_type)
    }

    /// Checks that Category `id` is a leaf, then returns the Uncategorised Category its Splits move
    /// to before the delete, creating it if needed. Refuses a missing or non-leaf Category before
    /// anything is created.
    pub fn prepare_delete(&mut self, id: u32) -> Result<u32, CategoryError> {
        let category = get(&self.categories, id).ok_or(CategoryError::NotFound)?;
        if !is_leaf(&self.categories, id) {
            return Err(CategoryError::NonLeafDeletion);
        }
        let category_type = category.category_type.clone();
        Ok(get_or_create_uncategorised(
            &mut self.categories,
            category_type,
        ))
    }

    /// Deletes leaf Category `id`; see [`delete_category`].
    pub fn delete(&mut self, id: u32) -> Result<(), CategoryError> {
        delete_category(&mut self.categories, id)
    }

    /// Replaces the whole tree. Only for a caller that has already checked its copy with the
    /// rules above, as Import does for Payees.
    pub fn replace(&mut self, rows: Vec<Category>) {
        self.categories = rows;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_applies_the_tree_rules_and_keeps_writes() {
        let mut service = CategoryService::new(default_categories());
        let parent = service.categories().len();
        // Food (6) is top level and Expense, so a child under it is accepted.
        let child = service
            .insert("Snacks".into(), Some(6), CategoryTypes::Expense)
            .expect("a leaf under an Expense parent is accepted");
        assert_eq!(service.categories().len(), parent + 1);
        // Salary (11) is Income, so an Expense child under it is refused and the tree is untouched.
        assert_eq!(
            service.insert("Bonus".into(), Some(11), CategoryTypes::Expense),
            Err(CategoryError::TypeMismatch)
        );
        assert_eq!(service.categories().len(), parent + 1);
        service.delete(child).expect("a leaf can be deleted");
        assert!(get(service.categories(), child).is_none());
    }
}
