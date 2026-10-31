//! The Ledger's Categories are a read copy of the Categories port (#858): a Client refreshes it from
//! the port after each Categories write and on rollback ([`Ledger::refresh_categories`]), for the
//! Views not yet modules: the Categories page, Transactions, Budgets, Documents and the Dashboard. The Categories writers still run as `LedgerCommand`s, which a Client forwards to the
//! port, until the Categories page writes it itself (#859) and Category reassign moves to
//! `lib-reassign` (#813). The copy goes when the last reader has its own port.

use lib_categories::Category;

use crate::Ledger;

impl Ledger {
    /// Replaces the read copy of the Categories with what the Categories port holds, in the port's
    /// order (tree order). Returns whether anything changed, so a Client announces only a real
    /// change.
    pub fn refresh_categories(&mut self, categories: Vec<Category>) -> bool {
        if self.categories == categories {
            return false;
        }
        self.categories = categories;
        true
    }
}

#[cfg(test)]
mod tests {
    use lib_core::{CategoryId, CategoryTypes};

    use super::*;

    #[test]
    fn a_refresh_replaces_the_copy_and_says_whether_it_changed() {
        let mut ledger = Ledger::stub();
        let mut categories = ledger.categories().to_vec();
        assert!(!ledger.refresh_categories(categories.clone()));

        categories.push(Category {
            id: CategoryId::new(),
            name: "Dividends".into(),
            parent: None,
            category_type: CategoryTypes::Income,
        });
        assert!(ledger.refresh_categories(categories.clone()));
        assert_eq!(ledger.categories(), categories);
    }
}
