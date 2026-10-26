//! The in-memory port and the stub seed, for tests and UI-first work. It holds its rows in a plain
//! field, in the order they were added, so a test can set up or read state directly.

use crate::{Category, CategoryRepository, default_categories, tree_order};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryCategories {
    /// The rows in the order they were added; [`CategoryRepository::categories`] lists them in
    /// tree order.
    pub categories: Vec<Category>,
}

impl InMemoryCategories {
    /// The stub seed: the stub Ledger's Categories (see [`default_categories`]). Ids match
    /// `lib-ledger`'s stub, so its Splits, Bill Plans, Payees and Budgets still resolve theirs.
    pub fn stub() -> Self {
        Self {
            categories: default_categories(),
        }
    }
}

impl CategoryRepository for InMemoryCategories {
    fn categories(&self) -> Vec<Category> {
        tree_order(&self.categories)
    }

    fn upsert_category(&mut self, category: Category) {
        match self.categories.iter_mut().find(|row| row.id == category.id) {
            Some(row) => *row = category,
            None => self.categories.push(category),
        }
    }
}
