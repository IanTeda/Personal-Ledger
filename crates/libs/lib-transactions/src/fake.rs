//! The in-memory Transactions ports and the stub seed, for tests and UI-first work. Each port holds
//! its rows in plain fields so a test can set up or read state directly.

mod seed;

use lib_core::{TransactionId, UnitId};

pub use seed::{DEFAULT_SEED, HISTORY_DAYS, SeedAccount, SeedNames, stub_transactions};

use crate::{
    Transaction, TransactionAccount, TransactionAccounts, TransactionCategories,
    TransactionCategory, TransactionPayee, TransactionPayees, TransactionRepository,
    TransactionTag, TransactionTags, TransactionUnit, TransactionUnits,
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
    pub accounts: Vec<TransactionAccount>,
    pub categories: Vec<TransactionCategory>,
    pub payees: Vec<TransactionPayee>,
    pub tags: Vec<TransactionTag>,
    pub units: Vec<TransactionUnit>,
}

impl TransactionAccounts for InMemoryTransactionReads {
    fn transaction_accounts(&self) -> Vec<TransactionAccount> {
        self.accounts.clone()
    }
}

impl TransactionCategories for InMemoryTransactionReads {
    fn transaction_categories(&self) -> Vec<TransactionCategory> {
        self.categories.clone()
    }
}

impl TransactionPayees for InMemoryTransactionReads {
    fn transaction_payees(&self) -> Vec<TransactionPayee> {
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
