//! The Transaction Import adapter: `lib-transaction-import`'s atomic port over the Payees and
//! Transactions stores (ADR-0046), after the Bill Payments adapter. It holds no rows of its own. An
//! Import's new Payees, with their aliases, go to the Payees store first, then its Transactions to
//! the Transactions store, and every write joins the [`PendingWrites`] every adapter shares, so the
//! bin's one commit after the call sends them as one transaction. The plan is checked against the
//! stored Payees before anything is written, so a refused plan writes nothing.
//!
//! It answers the module's Payee read from the same Payees store; the Accounts and Categories
//! adapters answer its other reads from their rows (`accounts.rs`, `categories.rs`).
//!
//! [`PendingWrites`]: crate::PendingWrites

use lib_payees::Payee;
use lib_transaction_import::{
    ImportCommitted, ImportPayees, ImportPlan, ImportRefused, TransactionImport,
};

use super::store::{SharedPayeesStore, SharedTransactionsStore};

/// The import port over the stores the Payees and Transactions modules also hold.
pub struct TransactionImportAdapter {
    payees: SharedPayeesStore,
    transactions: SharedTransactionsStore,
}

impl TransactionImportAdapter {
    pub fn new(payees: SharedPayeesStore, transactions: SharedTransactionsStore) -> Self {
        Self {
            payees,
            transactions,
        }
    }
}

impl TransactionImport for TransactionImportAdapter {
    /// The Payees first: each Transaction's Split names one, which must be stored before it.
    fn commit(&mut self, plan: ImportPlan) -> Result<ImportCommitted, ImportRefused> {
        let committed = plan.check(&self.payees.borrow().payees().payees())?;
        {
            let mut store = self.payees.borrow_mut();
            for payee in plan.payees {
                store.payees_mut().upsert_payee(payee);
            }
        }
        let mut store = self.transactions.borrow_mut();
        for transaction in plan.transactions {
            store.upsert_transaction(transaction);
        }
        Ok(committed)
    }
}

impl ImportPayees for TransactionImportAdapter {
    fn import_payees(&self) -> Vec<Payee> {
        self.payees.borrow().payees().payees()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::mpsc;

    use lib_transaction_import::contract::{self, ImportFactory, Stored};
    use lib_transactions::{Transaction, TransactionRepository as _};
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::commit::Commit;
    use crate::writer::WriteFailure;
    use crate::{PayeesAdapter, PendingWrites, TransactionsAdapter, Writer};

    /// Both adapters on one test database and one pending batch, as start-up opens them. The
    /// stub's Units, Accounts and Categories are stored first, which a Payee and a Split name.
    struct AdapterFactory {
        writer: Writer,
        pending: PendingWrites,
        failures: mpsc::Receiver<WriteFailure>,
    }

    impl AdapterFactory {
        fn new(options: SqliteConnectOptions) -> Self {
            let (writer, failures) = Writer::spawn(options).expect("the test database opens");
            crate::test_support::store_stub_references(&writer);
            Self {
                pending: PendingWrites::new(writer.clone()),
                writer,
                failures,
            }
        }
    }

    impl ImportFactory for AdapterFactory {
        type Ports = TransactionImportAdapter;

        fn create(
            &mut self,
            payees: Vec<Payee>,
            transactions: Vec<Transaction>,
        ) -> TransactionImportAdapter {
            PayeesAdapter::seed(&self.writer, payees).expect("the Payees are written");
            TransactionsAdapter::seed(&self.writer, transactions)
                .expect("the Transactions are written");
            let payees = PayeesAdapter::load(self.pending.clone()).expect("the Payees load");
            let transactions =
                TransactionsAdapter::load(self.pending.clone()).expect("the Transactions load");
            TransactionImportAdapter::new(
                Rc::new(RefCell::new(payees)),
                Rc::new(RefCell::new(transactions)),
            )
        }

        /// Commits once, settles the writes, and reads back what is stored with fresh adapters,
        /// as a restart would.
        fn stored(&mut self, _ports: TransactionImportAdapter) -> Stored {
            self.pending.commit();
            self.writer.flush().expect("the writer is running");
            let failures = std::iter::from_fn(|| self.failures.try_recv().ok()).count();
            assert_eq!(failures, 0, "a write failed");
            let fresh = PendingWrites::new(self.writer.clone());
            let payees = PayeesAdapter::load(fresh.clone()).expect("the Payees load");
            let transactions = TransactionsAdapter::load(fresh).expect("the Transactions load");
            Stored::new(payees.payees().payees(), transactions.transactions())
        }
    }

    /// The adapter over the in-memory stores `Stores::fake` bundles, as a Client test builds it.
    #[derive(Default)]
    struct FakeStoresFactory {
        payees: Rc<RefCell<lib_payees::fake::InMemoryPorts>>,
        transactions: Rc<RefCell<lib_transactions::fake::InMemoryTransactions>>,
    }

    impl ImportFactory for FakeStoresFactory {
        type Ports = TransactionImportAdapter;

        fn create(
            &mut self,
            payees: Vec<Payee>,
            transactions: Vec<Transaction>,
        ) -> TransactionImportAdapter {
            self.payees.borrow_mut().payees.payees = payees;
            self.transactions.borrow_mut().transactions = transactions;
            TransactionImportAdapter::new(self.payees.clone(), self.transactions.clone())
        }

        fn stored(&mut self, _ports: TransactionImportAdapter) -> Stored {
            Stored::new(
                self.payees.borrow().payees.payees.clone(),
                self.transactions.borrow().transactions.clone(),
            )
        }
    }

    #[test]
    fn over_the_fake_stores_a_commit_lands_and_a_refused_plan_writes_nothing() {
        contract::a_commit_lands_the_payees_their_aliases_and_the_transactions(
            &mut FakeStoresFactory::default(),
        );
        contract::a_refused_plan_writes_nothing(&mut FakeStoresFactory::default());
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_commit_lands_the_payees_their_aliases_and_the_transactions(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_commit_lands_the_payees_their_aliases_and_the_transactions(
            &mut AdapterFactory::new(options),
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_refused_plan_writes_nothing(_: SqlitePoolOptions, options: SqliteConnectOptions) {
        contract::a_refused_plan_writes_nothing(&mut AdapterFactory::new(options));
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn the_payee_read_answers_every_stored_payee(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        let mut factory = AdapterFactory::new(options);
        let mut payees = lib_payees::default_payees();
        payees[0].is_active = false;
        let import = factory.create(payees.clone(), Vec::new());

        let mut read = import.import_payees();
        read.sort_by_key(|payee| payee.id);
        payees.sort_by_key(|payee| payee.id);

        assert_eq!(read, payees, "inactive Payees too: their names stay taken");
    }
}
