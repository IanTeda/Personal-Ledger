//! The Transactions ports as the Desktop holds them (#774, #923): [`DesktopPorts`], what the
//! page's Presenter reads, answered from the stores the Open Ledger assembles (`LedgerStores`):
//! the Transactions store, and the other domains through `lib_database`'s
//! `StoredTransactionReads`. Every writer of Transactions is a module writing the store, or a
//! cross-domain module's port over it (#1006); the input it is made in announces it (#1081), and
//! the Ledger keeps no read copy (#1028).

use std::cell::RefCell;
use std::rc::Rc;

use lib_core::UnitId;
use lib_database::{SharedTransactionsStore, StoredTransactionReads};
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
