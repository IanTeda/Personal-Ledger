//! The Transactions port as the Desktop holds it (#774): the forwarding shim that saves today's
//! writers' Transaction changes through the store `shell::router::build` is given (`LedgerSetup`).
//!
//! The Transactions View and every writer of Transactions are still today's code: each runs as a
//! `LedgerCommand` on the Ledger and records a `LedgerChange`. [`forward`] sends the Transaction
//! ones to the store in the same commit as the command, and the Ledger's Transactions become a read
//! copy refreshed from the store after each commit and on rollback. Each writer leaves the shim
//! when it migrates; the read copy goes when the last reader reads through the port.

use lib_database::TransactionsStore;
use lib_ledger::LedgerChange;

/// Sends each Transaction row in `changes` to `store`, in the order recorded. The writers it
/// carries: Account delete, Import commit, Tag delete and merge, and Category reassign. Every
/// other change is another domain's, which nothing persists yet.
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
