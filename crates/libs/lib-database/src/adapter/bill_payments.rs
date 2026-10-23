//! The Bill Payments adapter: `lib-bill-payments`' atomic port over the Transactions and Bills
//! stores (ADR-0046). It holds no rows of its own. A payment's Transaction goes to the Transactions
//! store and its entry to the Bills store, and both writes join the [`PendingWrites`] every adapter
//! shares, so the bin's one commit after the call sends them as one transaction: a refused entry
//! takes the Transaction down with it.
//!
//! [`PendingWrites`]: crate::PendingWrites

use lib_bill_payments::BillPayments;
use lib_bills::{BillPlan, BillScheduleEntry, EntryId};
use lib_core::BillPlanId;
use lib_transactions::Transaction;

use super::store::{SharedBillsStore, SharedTransactionsStore};

/// The payments port over the stores the Transactions and Bills modules also hold.
pub struct BillPaymentsAdapter {
    transactions: SharedTransactionsStore,
    bills: SharedBillsStore,
}

impl BillPaymentsAdapter {
    pub fn new(transactions: SharedTransactionsStore, bills: SharedBillsStore) -> Self {
        Self {
            transactions,
            bills,
        }
    }
}

impl BillPayments for BillPaymentsAdapter {
    fn bill_plan(&self, id: BillPlanId) -> Option<BillPlan> {
        self.bills.borrow().plans().bill_plan(id)
    }

    fn entry(&self, id: EntryId) -> Option<BillScheduleEntry> {
        self.bills.borrow().entries().entry(id)
    }

    /// The Transaction first: the entry's Paid link is its Split's, which must be stored before it.
    fn record_payment(&mut self, transaction: Transaction, entry: BillScheduleEntry) {
        self.transactions
            .borrow_mut()
            .upsert_transaction(transaction);
        self.bills.borrow_mut().entries_mut().upsert_entry(entry);
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;
    use std::sync::mpsc;

    use lib_bill_payments::contract::{self, PaymentsFactory, Stored};
    use lib_bills::fake::InMemoryBills;
    use lib_bills::{BillPlanRepository as _, BillScheduleRepository as _};
    use lib_transactions::TransactionRepository as _;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;
    use crate::commit::Commit;
    use crate::writer::WriteFailure;
    use crate::{BillsAdapter, PendingWrites, TransactionsAdapter, UnitsAdapter, Writer};

    /// Both adapters on one test database and one pending batch, as start-up opens them. The
    /// stub's Units are stored first, which a Bill Plan's Unit names.
    struct AdapterFactory {
        writer: Writer,
        pending: PendingWrites,
        failures: mpsc::Receiver<WriteFailure>,
    }

    impl AdapterFactory {
        fn new(options: SqliteConnectOptions) -> Self {
            let (writer, failures) = Writer::spawn(options).expect("the test database opens");
            UnitsAdapter::seed_from(
                &writer,
                lib_units::fake::InMemoryPorts::stub(chrono::Utc::now()),
            )
            .expect("the stub Units are written");
            Self {
                pending: PendingWrites::new(writer.clone()),
                writer,
                failures,
            }
        }

        /// Commits, settles the writes, and reads back what is stored with fresh adapters, as a
        /// restart would.
        fn settle(&mut self) -> (Stored, usize) {
            self.pending.commit();
            self.writer.flush().expect("the writer is running");
            let failures = std::iter::from_fn(|| self.failures.try_recv().ok()).count();
            let fresh = PendingWrites::new(self.writer.clone());
            let transactions =
                TransactionsAdapter::load(fresh.clone()).expect("the Transactions load");
            let bills = BillsAdapter::load(fresh).expect("the Bills load");
            let stored = Stored::new(
                transactions.transactions(),
                bills.bill_plans(),
                bills.entries(),
            );
            (stored, failures)
        }
    }

    impl PaymentsFactory for AdapterFactory {
        type Ports = BillPaymentsAdapter;

        fn create(
            &mut self,
            bills: InMemoryBills,
            transactions: Vec<Transaction>,
        ) -> BillPaymentsAdapter {
            TransactionsAdapter::seed(&self.writer, transactions)
                .expect("the Transactions are written");
            BillsAdapter::seed(&self.writer, bills.plans, bills.entries)
                .expect("the Bills are written");
            let transactions =
                TransactionsAdapter::load(self.pending.clone()).expect("the Transactions load");
            let bills = BillsAdapter::load(self.pending.clone()).expect("the Bills load");
            BillPaymentsAdapter::new(
                Rc::new(RefCell::new(transactions)),
                Rc::new(RefCell::new(bills)),
            )
        }

        fn stored(&mut self, _ports: BillPaymentsAdapter) -> Stored {
            let (stored, failures) = self.settle();
            assert_eq!(failures, 0, "a write failed");
            stored
        }

        fn break_entry_writes(&mut self, _ports: &mut BillPaymentsAdapter) {
            self.writer
                .run(|pool| async move {
                    for event in ["INSERT", "UPDATE"] {
                        sqlx::query(&format!(
                            "CREATE TRIGGER refuse_entry_{event} BEFORE {event} ON \
                             bill_schedule_entries BEGIN SELECT RAISE(ABORT, 'writes are \
                             refused'); END"
                        ))
                        .execute(&pool)
                        .await?;
                    }
                    Ok(())
                })
                .expect("the triggers are created");
        }

        fn recover(&mut self, _ports: BillPaymentsAdapter) -> (Stored, usize) {
            self.settle()
        }
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_payment_lands_the_transaction_its_split_and_the_paid_link(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_payment_lands_the_transaction_its_split_and_the_paid_link(
            &mut AdapterFactory::new(options),
        );
    }

    #[sqlx::test(migrations = "migrations/client")]
    async fn a_failing_payment_lands_neither_row(
        _: SqlitePoolOptions,
        options: SqliteConnectOptions,
    ) {
        contract::a_failing_payment_lands_neither_row(&mut AdapterFactory::new(options));
    }
}
