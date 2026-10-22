//! The port the Transactions are stored through. It is synchronous (ADR-0046): an adapter serves
//! reads from memory and queues each write, so a write never fails here; a failed save is the
//! adapter's to report. `fake` implements it in memory.
//!
//! Ports only, for now: the Transactions View and every writer of Transactions are still today's
//! code, which reaches this port through the Desktop's forwarding shim (#774). Interactors,
//! Presenters and a Router trait come when the View migrates.

use lib_core::TransactionId;

use crate::Transaction;

/// The Transactions on the Ledger, each with its Splits.
pub trait TransactionRepository {
    /// Every Transaction, in no particular order.
    fn transactions(&self) -> Vec<Transaction>;

    fn transaction(&self, id: TransactionId) -> Option<Transaction> {
        self.transactions()
            .into_iter()
            .find(|transaction| transaction.id == id)
    }

    /// Writes `transaction` with its Splits, replacing the row with its id or adding it.
    fn upsert_transaction(&mut self, transaction: Transaction);

    /// Removes the Transaction and its Splits. A Transaction not stored is no error.
    fn delete_transaction(&mut self, id: TransactionId);
}
