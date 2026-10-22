//! The Transactions port as the Desktop holds it (#774): the store `build_shell` is given and the
//! forwarding shim that saves today's writers' Transaction changes through it.
//!
//! The Transactions View and every writer of Transactions are still today's code: each runs as a
//! `LedgerCommand` on the Ledger and records a `LedgerChange`. [`forward`] sends the Transaction
//! ones to the store in the same commit as the command, and the Ledger's Transactions become a read
//! copy refreshed from the store after each commit and on rollback. Each writer leaves the shim
//! when it migrates; the read copy goes when the last reader reads through the port.

use std::rc::Rc;

use lib_database::{Commit, TransactionsStore};
use lib_ledger::LedgerChange;

/// What `build_shell` hands the Ledger: where its Transactions are kept, and the commit that ends
/// each call's writes, every adapter's at once.
pub struct TransactionsSetup {
    pub store: Box<dyn TransactionsStore>,
    pub commit: Rc<dyn Commit>,
}

impl TransactionsSetup {
    /// The stub Ledger's Transactions as of `today`, in memory, for a test that builds `Shell`
    /// directly.
    #[cfg(test)]
    pub fn stub(today: chrono::NaiveDate) -> Self {
        Self {
            store: Box::new(lib_transactions::fake::InMemoryTransactions {
                transactions: lib_ledger::Ledger::stub_on(today).transactions().to_vec(),
            }),
            commit: Rc::new(lib_database::InMemoryCommit),
        }
    }
}

/// Sends each Transaction row in `changes` to `store`, in the order recorded. The writers it
/// carries: Account delete, Import commit, Tag delete and merge, Category reassign and Bills'
/// Pay. Every other change is another domain's, which nothing persists yet.
pub fn forward(store: &mut dyn TransactionsStore, changes: &[LedgerChange]) {
    let port = store.transactions_mut();
    for change in changes {
        match change {
            LedgerChange::UpsertTransaction(transaction) => {
                port.upsert_transaction(transaction.clone());
            }
            LedgerChange::DeleteTransaction(id) => port.delete_transaction(*id),
            _ => {}
        }
    }
}
