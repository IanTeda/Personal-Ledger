//! `lib-transactions`' ports answered over the stores: the Transactions from the Transactions
//! store, and the other domains' rows the page shows through [`StoredTransactionReads`] (#774,
//! #923, #1083).

use lib_core::UnitId;
use lib_transactions::{
    Transaction, TransactionAccount, TransactionAccounts, TransactionCategories,
    TransactionCategory, TransactionPayee, TransactionPayees, TransactionTag, TransactionTags,
    TransactionUnits, TransactionsPorts,
};

use crate::adapter::store::{SharedTransactionsStore, StoredTransactionReads};

/// [`TransactionsPorts`] over the shared Transactions store, borrowed for each call, and the
/// reads over the stores of the domains a Transaction names. The port traits are
/// `TransactionsPorts`' supertraits, so the other domains' reads are only forwarded.
pub struct StoredTransactionsPorts {
    transactions: SharedTransactionsStore,
    reads: StoredTransactionReads,
}

impl StoredTransactionsPorts {
    pub fn new(transactions: SharedTransactionsStore, reads: StoredTransactionReads) -> Self {
        Self {
            transactions,
            reads,
        }
    }
}

impl TransactionsPorts for StoredTransactionsPorts {
    fn all_transactions(&self) -> Vec<Transaction> {
        self.transactions.borrow().all_transactions()
    }
}

impl TransactionAccounts for StoredTransactionsPorts {
    fn transaction_accounts(&self) -> Vec<TransactionAccount> {
        self.reads.transaction_accounts()
    }
}

impl TransactionCategories for StoredTransactionsPorts {
    fn transaction_categories(&self) -> Vec<TransactionCategory> {
        self.reads.transaction_categories()
    }
}

impl TransactionPayees for StoredTransactionsPorts {
    fn transaction_payees(&self) -> Vec<TransactionPayee> {
        self.reads.transaction_payees()
    }
}

impl TransactionTags for StoredTransactionsPorts {
    fn transaction_tags(&self) -> Vec<TransactionTag> {
        self.reads.transaction_tags()
    }
}

impl TransactionUnits for StoredTransactionsPorts {
    fn transaction_unit_code(&self, id: UnitId) -> Option<String> {
        self.reads.transaction_unit_code(id)
    }
}

#[cfg(test)]
mod tests {
    use crate::ports::test_support::{fakes, file};
    use crate::{Domain, LedgerPorts};

    #[test]
    fn a_transactions_read_reports_nothing_written_and_a_store_write_reports_transactions() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        for ledger in [fakes(), file(dir.path())] {
            let LedgerPorts {
                stores,
                transactions,
                commit,
                ..
            } = ledger;
            let rows = transactions.borrow().all_transactions();
            assert!(!rows.is_empty(), "the Sample Ledger has Transactions");
            assert!(commit.commit().is_empty(), "a read writes nothing");

            // Every writer of Transactions is a module writing the store: the store's recorder
            // names the domain.
            let first = rows.into_iter().next().expect("a Transaction");
            stores
                .transactions
                .borrow_mut()
                .transactions_mut()
                .upsert_transaction(first);

            assert_eq!(commit.commit().wrote, [Domain::Transactions]);
        }
    }
}
