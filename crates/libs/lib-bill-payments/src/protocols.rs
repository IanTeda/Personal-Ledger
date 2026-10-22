//! The port Pay it directly writes through, and the Router the Pay Dialog closes through.
//!
//! [`BillPayments`] is the module's one atomic port: a payment is a Transaction and the Bill
//! Schedule entry it settles, written together or not at all. It is synchronous (ADR-0046): an
//! adapter serves its reads from memory and queues both writes in the batch every adapter shares,
//! so the bin's one commit after the call lands them as one transaction, and a failed save is the
//! adapter's to report. `fake` implements it in memory.
//!
//! The Pay Dialog's Match panel and its wording read the Bills ports (`lib_bills::SharedPorts`):
//! Match writes only the entry, so it is `lib-bills`' Interactor, not this module's.

use std::cell::RefCell;
use std::rc::Rc;

use lib_bills::{BillPlan, BillScheduleEntry, EntryId};
use lib_core::{BillPlanId, TransactionId};
use lib_transactions::Transaction;

/// The Bills a payment checks, and the one write that records it.
pub trait BillPayments {
    /// Bill Plan `id`, which a payment's Transaction is drawn from.
    fn bill_plan(&self, id: BillPlanId) -> Option<BillPlan>;

    /// The entry for a Bill Plan and due date, which must be open to pay.
    fn entry(&self, id: EntryId) -> Option<BillScheduleEntry>;

    /// Writes `transaction` with its Splits and `entry`, Paid by one of them, as one write: both
    /// land or neither does. `entry` replaces the one for its Bill Plan and due date.
    fn record_payment(&mut self, transaction: Transaction, entry: BillScheduleEntry);
}

/// The payments port as the Pay Dialog holds it, shared with the Client that commits its writes.
/// Single-threaded, as both Clients' UI threads are.
pub type SharedPayments = Rc<RefCell<dyn BillPayments>>;

/// What a confirmed Pay Dialog did, handed to the Client when the Dialog closes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayOutcome {
    /// Paid directly by the new Transaction `id`.
    Paid(TransactionId),
    /// Matched to an existing Transaction's Split.
    Matched,
}

/// Navigation out of the Pay Dialog's Presenter, implemented by each Client and passed into each
/// `handle` call, as `lib-bills`' Router is.
pub trait BillPaymentsRouter {
    /// Closes the Pay Dialog. `outcome` is what its confirm did, `None` when it was cancelled.
    fn close_dialog(&mut self, outcome: Option<PayOutcome>);
}
