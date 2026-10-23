//! The in-memory payments port, for tests and UI-first work, and a Router that records what the
//! Pay Dialog's Presenter asked for. Each holds its rows in plain fields so a test can set up or
//! read state directly.

use std::cell::RefCell;
use std::rc::Rc;

use lib_bills::fake::{InMemoryBills, InMemoryPorts};
use lib_bills::{
    BillPlan, BillPlanRepository, BillScheduleEntry, BillScheduleRepository, BillSplit, EntryId,
    SplitRef,
};
use lib_core::BillPlanId;
use lib_toast::{Notice, Notify};
use lib_transactions::fake::InMemoryTransactions;
use lib_transactions::{Transaction, TransactionRepository};

use crate::protocols::{BillPayments, BillPaymentsRouter, PayOutcome, SharedPayments};

/// A payment's two stores in memory, the Transactions and the Bills, written together.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryPayments {
    pub transactions: InMemoryTransactions,
    pub bills: InMemoryBills,
}

impl BillPayments for InMemoryPayments {
    fn bill_plan(&self, id: BillPlanId) -> Option<BillPlan> {
        self.bills.bill_plan(id)
    }

    fn entry(&self, id: EntryId) -> Option<BillScheduleEntry> {
        self.bills.entry(id)
    }

    fn record_payment(&mut self, transaction: Transaction, entry: BillScheduleEntry) {
        self.transactions.upsert_transaction(transaction);
        self.bills.upsert_entry(entry);
    }
}

/// The payments port over `lib-bills`' in-memory ports, as the Pay Dialog's Presenter tests share
/// them: the entry goes to their Bill Schedule and the Transaction's Splits to the Splits they
/// read, so the Bills page and a Match see the payment at once.
#[derive(Debug, Clone)]
pub struct PortsPayments(pub Rc<RefCell<InMemoryPorts>>);

impl PortsPayments {
    /// This port shared as the Presenter holds it.
    pub fn shared(ports: &Rc<RefCell<InMemoryPorts>>) -> SharedPayments {
        Rc::new(RefCell::new(Self(ports.clone())))
    }
}

impl BillPayments for PortsPayments {
    fn bill_plan(&self, id: BillPlanId) -> Option<BillPlan> {
        self.0.borrow().bills.bill_plan(id)
    }

    fn entry(&self, id: EntryId) -> Option<BillScheduleEntry> {
        self.0.borrow().bills.entry(id)
    }

    fn record_payment(&mut self, transaction: Transaction, entry: BillScheduleEntry) {
        let mut ports = self.0.borrow_mut();
        let splits = &mut ports.reads.splits;
        splits.retain(|row| row.split.transaction_id != transaction.id);
        splits.extend(
            transaction
                .splits
                .into_iter()
                .enumerate()
                .map(|(split_index, split)| BillSplit {
                    split: SplitRef {
                        transaction_id: transaction.id,
                        split_index,
                    },
                    date: transaction.date,
                    account_id: transaction.account_id,
                    description: transaction.description.clone(),
                    amount: split.amount,
                    category_id: split.category_id,
                    payee_id: split.payee_id,
                }),
        );
        ports.bills.upsert_entry(entry);
    }
}

/// A Router that records each close and each Notice, in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecordingRouter {
    pub closed: Vec<Option<PayOutcome>>,
    pub notices: Vec<Notice>,
}

impl BillPaymentsRouter for RecordingRouter {
    fn close_dialog(&mut self, outcome: Option<PayOutcome>) {
        self.closed.push(outcome);
    }
}

impl Notify for RecordingRouter {
    fn notify(&mut self, notice: Notice) {
        self.notices.push(notice);
    }
}
