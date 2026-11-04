//! The Transactions ports as the Desktop holds them (#774, #923).
//!
//! - [`DesktopPorts`]: what the page's Presenter reads, answered from the stores `shell::router::build`
//!   is given (`LedgerSetup`) rather than from the Ledger read copy: the Transactions store, and
//!   the other domains through `lib_database`'s `StoredTransactionReads`.
//! - [`forward`]: the forwarding shim that saves today's writers' Transaction changes through the
//!   Transactions store. Every writer of Transactions is still today's code: each runs as a
//!   `LedgerCommand` on the Ledger and records a `LedgerChange`, which [`forward`] sends to the store
//!   in the same commit as the command. The Ledger's Transactions stay a read copy refreshed from the
//!   store for the readers not yet on the port. Each writer leaves the shim when it migrates; the
//!   read copy goes when the last reader reads through the port.

use std::cell::RefCell;
use std::rc::Rc;

use lib_core::UnitId;
use lib_database::{SharedTransactionsStore, StoredTransactionReads, TransactionsStore};
use lib_ledger::LedgerChange;
use lib_transactions::{
    SharedPorts, TransactionAccount, TransactionAccounts, TransactionCategories,
    TransactionCategory, TransactionPayee, TransactionPayees, TransactionTag, TransactionTags,
    TransactionUnits, TransactionsPorts,
};

/// The Transactions store, and the other domains' rows read through `reads`, each store borrowed
/// for one read, since a store is shared behind a `RefCell` whose guard can't be lent out of it.
/// The port traits are `TransactionsPorts`' supertraits, so the other domains' reads are only
/// forwarded: `lib_database` maps them (#998).
pub struct DesktopPorts {
    pub transactions: SharedTransactionsStore,
    pub reads: StoredTransactionReads,
}

impl DesktopPorts {
    pub fn shared(
        transactions: SharedTransactionsStore,
        reads: StoredTransactionReads,
    ) -> SharedPorts {
        Rc::new(RefCell::new(Self {
            transactions,
            reads,
        }))
    }
}

impl TransactionsPorts for DesktopPorts {
    fn all_transactions(&self) -> Vec<lib_transactions::Transaction> {
        self.transactions.borrow().all_transactions()
    }
}

impl TransactionAccounts for DesktopPorts {
    fn transaction_accounts(&self) -> Vec<TransactionAccount> {
        self.reads.transaction_accounts()
    }
}

impl TransactionCategories for DesktopPorts {
    fn transaction_categories(&self) -> Vec<TransactionCategory> {
        self.reads.transaction_categories()
    }
}

impl TransactionPayees for DesktopPorts {
    fn transaction_payees(&self) -> Vec<TransactionPayee> {
        self.reads.transaction_payees()
    }
}

impl TransactionTags for DesktopPorts {
    fn transaction_tags(&self) -> Vec<TransactionTag> {
        self.reads.transaction_tags()
    }
}

impl TransactionUnits for DesktopPorts {
    fn transaction_unit_code(&self, id: UnitId) -> Option<String> {
        self.reads.transaction_unit_code(id)
    }
}

/// Sends each Transaction row `changes` upserts to `store`, in the order recorded. The writers it
/// carries: Tag delete and merge, and Category reassign, until `lib-reassign` (#813); none deletes
/// a Transaction, since Account delete writes through `lib-account-deletion`'s port (#854) and the
/// Import through `lib-transaction-import`'s (#943). Every
/// other change is another domain's, which nothing persists yet.
pub fn forward(store: &mut dyn TransactionsStore, changes: &[LedgerChange]) {
    let port = store.transactions_mut();
    for change in changes {
        if let LedgerChange::UpsertTransaction(transaction) = change {
            port.upsert_transaction(transaction.clone());
        }
    }
}
