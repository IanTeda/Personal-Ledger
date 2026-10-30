//! The in-memory Transactions ports and the stub seed, for tests and UI-first work. Each port holds
//! its rows in plain fields so a test can set up or read state directly.

mod seed;

use std::cell::RefCell;
use std::rc::Rc;

use lib_core::{TransactionId, UnitId};

pub use seed::{DEFAULT_SEED, HISTORY_DAYS, SeedAccount, SeedNames, stub_transactions};

use crate::protocols::SharedPorts;
use crate::{
    Transaction, TransactionAccount, TransactionAccounts, TransactionCategories,
    TransactionCategory, TransactionPayee, TransactionPayees, TransactionRepository,
    TransactionTag, TransactionTags, TransactionUnit, TransactionUnits, TransactionsPorts,
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
    fn transaction_accounts(&self) -> Vec<TransactionAccount> {
        self.reads.transaction_accounts()
    }
}

impl TransactionCategories for InMemoryPorts {
    fn transaction_categories(&self) -> Vec<TransactionCategory> {
        self.reads.transaction_categories()
    }
}

impl TransactionPayees for InMemoryPorts {
    fn transaction_payees(&self) -> Vec<TransactionPayee> {
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
