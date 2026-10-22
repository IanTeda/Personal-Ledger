//! The in-memory Transactions port and the stub seed, for tests and UI-first work. The port holds
//! its rows in a plain field so a test can set up or read state directly.

mod seed;

use lib_core::TransactionId;

pub use seed::{DEFAULT_SEED, HISTORY_DAYS, SeedAccount, SeedNames, stub_transactions};

use crate::{Transaction, TransactionRepository};

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
