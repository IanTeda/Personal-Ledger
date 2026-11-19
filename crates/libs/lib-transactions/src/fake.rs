//! The in-memory Transactions ports and the sample seed, for tests and UI-first work. Each port holds
//! its rows in plain fields so a test can set up or read state directly.

mod booked;
mod seed;

use lib_core::AccountRef;
use lib_core::CategoryRef;
use lib_core::PayeeRef;
use std::cell::RefCell;
use std::rc::Rc;

use lib_core::{TransactionId, UnitId, UnitRef};

pub use booked::{
    FIRST_BOOKED, bill_transactions, booked_sample_transactions, document_transactions,
    sample_transaction_count,
};
pub use seed::{DEFAULT_SEED, HISTORY_DAYS, SeedAccount, SeedNames, sample_transactions};

use crate::protocols::SharedPorts;
use crate::{
    Transaction, TransactionAccounts, TransactionCategories, TransactionPayees,
    TransactionRepository, TransactionTag, TransactionTags, TransactionUnits, TransactionsPorts,
};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryTransactions {
    pub transactions: Vec<Transaction>,
}

impl TransactionRepository for InMemoryTransactions {
    fn transactions(&self) -> Vec<Transaction> {
        self.transactions.clone()
    }

    fn upsert_transaction(&mut self, transaction: Transaction) {
        match self
            .transactions
            .iter_mut()
            .find(|row| row.id == transaction.id)
        {
            Some(row) => *row = transaction,
            None => self.transactions.push(transaction),
        }
    }

    fn delete_transaction(&mut self, id: TransactionId) {
        self.transactions.retain(|row| row.id != id);
    }
}

/// The other domains' rows the Transactions read, held in memory as they are answered: the
/// Categories already in tree order with their paths.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryTransactionReads {
    pub accounts: Vec<AccountRef>,
    pub categories: Vec<CategoryRef>,
    pub payees: Vec<PayeeRef>,
    pub tags: Vec<TransactionTag>,
    pub units: Vec<UnitRef>,
}

impl TransactionAccounts for InMemoryTransactionReads {
    fn transaction_accounts(&self) -> Vec<AccountRef> {
        self.accounts.clone()
    }
}

impl TransactionCategories for InMemoryTransactionReads {
    fn transaction_categories(&self) -> Vec<CategoryRef> {
        tree_order(&self.categories)
    }
}

/// Tree order ([`lib_core::tree_order_indices`]), as `lib_categories::tree_order` lists them. A row
/// the walk cannot reach (a missing parent or a cycle) follows the reachable rows in its stored
/// order, so none is dropped.
fn tree_order(categories: &[CategoryRef]) -> Vec<CategoryRef> {
    let reached = lib_core::tree_order_indices(categories);
    let mut placed = vec![false; categories.len()];
    for &index in &reached {
        placed[index] = true;
    }
    let unreached = (0..categories.len()).filter(|&index| !placed[index]);
    reached
        .into_iter()
        .chain(unreached)
        .map(|index| categories[index].clone())
        .collect()
}

impl TransactionPayees for InMemoryTransactionReads {
    fn transaction_payees(&self) -> Vec<PayeeRef> {
        self.payees.clone()
    }
}

impl TransactionTags for InMemoryTransactionReads {
    fn transaction_tags(&self) -> Vec<TransactionTag> {
        self.tags.clone()
    }
}

impl TransactionUnits for InMemoryTransactionReads {
    fn transaction_unit_code(&self, id: UnitId) -> Option<String> {
        self.units
            .iter()
            .find(|unit| unit.id == id)
            .map(|unit| unit.code.clone())
    }
}

/// Every port the Transactions page reads, in memory.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryPorts {
    pub transactions: InMemoryTransactions,
    pub reads: InMemoryTransactionReads,
}

impl InMemoryPorts {
    /// The ports shared, with the handle a test sets up and reads state back through.
    pub fn shared(self) -> (Rc<RefCell<Self>>, SharedPorts) {
        let ports = Rc::new(RefCell::new(self));
        let shared: SharedPorts = ports.clone();
        (ports, shared)
    }
}

impl TransactionsPorts for InMemoryPorts {
    fn all_transactions(&self) -> Vec<Transaction> {
        self.transactions.transactions()
    }
}

impl TransactionAccounts for InMemoryPorts {
    fn transaction_accounts(&self) -> Vec<AccountRef> {
        self.reads.transaction_accounts()
    }
}

impl TransactionCategories for InMemoryPorts {
    fn transaction_categories(&self) -> Vec<CategoryRef> {
        self.reads.transaction_categories()
    }
}

impl TransactionPayees for InMemoryPorts {
    fn transaction_payees(&self) -> Vec<PayeeRef> {
        self.reads.transaction_payees()
    }
}

impl TransactionTags for InMemoryPorts {
    fn transaction_tags(&self) -> Vec<TransactionTag> {
        self.reads.transaction_tags()
    }
}

impl TransactionUnits for InMemoryPorts {
    fn transaction_unit_code(&self, id: UnitId) -> Option<String> {
        self.reads.transaction_unit_code(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lib_core::{CategoryId, CategoryTypes};

    fn category(
        id: u32,
        path: &str,
        parent: Option<u32>,
        category_type: CategoryTypes,
    ) -> CategoryRef {
        let name = path.rsplit(" › ").next().unwrap_or(path).to_string();
        CategoryRef {
            id: CategoryId::seed(id),
            name,
            path: path.into(),
            parent: parent.map(CategoryId::seed),
            category_type,
            is_leaf: true,
        }
    }

    #[test]
    fn child_sorts_before_a_top_level_name_it_only_shares_a_prefix_with() {
        let reads = InMemoryTransactionReads {
            categories: vec![
                category(2, "Housing Extra", None, CategoryTypes::Expense),
                category(1, "Housing", None, CategoryTypes::Expense),
                category(3, "Housing › Rent", Some(1), CategoryTypes::Expense),
            ],
            ..Default::default()
        };

        let paths: Vec<String> = reads
            .transaction_categories()
            .into_iter()
            .map(|category| category.path)
            .collect();

        assert_eq!(paths, vec!["Housing", "Housing › Rent", "Housing Extra"]);
    }

    #[test]
    fn a_child_follows_its_own_parent_when_sibling_names_differ_only_in_case() {
        let reads = InMemoryTransactionReads {
            categories: vec![
                category(4, "food › b", Some(2), CategoryTypes::Expense),
                category(2, "food", None, CategoryTypes::Expense),
                category(3, "Food › a", Some(1), CategoryTypes::Expense),
                category(1, "Food", None, CategoryTypes::Expense),
            ],
            ..Default::default()
        };

        let paths: Vec<String> = reads
            .transaction_categories()
            .into_iter()
            .map(|category| category.path)
            .collect();

        assert_eq!(paths, vec!["Food", "Food › a", "food", "food › b",]);
    }

    #[test]
    fn a_row_the_walk_cannot_reach_follows_the_reachable_rows_in_its_stored_order() {
        let reads = InMemoryTransactionReads {
            categories: vec![
                category(3, "Orphan", Some(9), CategoryTypes::Expense),
                category(1, "Food", None, CategoryTypes::Expense),
                category(2, "Loop", Some(2), CategoryTypes::Expense),
            ],
            ..Default::default()
        };

        let paths: Vec<String> = reads
            .transaction_categories()
            .into_iter()
            .map(|category| category.path)
            .collect();

        assert_eq!(paths, vec!["Food", "Orphan", "Loop"]);
    }
}
