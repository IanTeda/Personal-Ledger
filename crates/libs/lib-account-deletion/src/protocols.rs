//! The port Account delete writes through, the reads its Dialog quotes, and the Router the Dialog
//! closes through.
//!
//! [`AccountDeletion`] is the module's one atomic port: an Account goes with the Transactions
//! booked to it and the Paid links they held, written together or not at all. It is synchronous
//! (ADR-0046): an adapter serves its reads from memory and queues every write in the batch every
//! adapter shares, so the bin's one commit after the call lands them as one transaction, and a
//! failed save is the adapter's to report. `fake` implements it in memory.
//!
//! It serves the reads the Interactor checks as owned values, rather than the Interactor taking
//! the Accounts, Transactions and Bills ports beside it: the adapter holds stores the other
//! modules share, so an Interactor given them all would borrow one twice (#779).
//!
//! What the Dialog only quotes, the counts and the Unit's code, it reads through
//! [`DeletionReads`] when it opens: the Budgets count is the `Ledger`'s until Budgets persist
//! (#811), which the port's adapter can't reach.

use std::cell::RefCell;
use std::rc::Rc;

use lib_accounts::{Account, AccountUsage};
use lib_bills::BillScheduleEntry;
use lib_core::{AccountId, TransactionId, UnitId};
use lib_toast::Notify;

/// Everything deleting one Account writes, as one write.
#[derive(Debug, Clone, PartialEq)]
pub struct AccountRemoval {
    pub account: AccountId,
    /// The Transactions booked to it, each going with its Splits.
    pub transactions: Vec<TransactionId>,
    /// The Bill Schedule entries those Transactions paid, each now Unresolved.
    pub unmatched: Vec<BillScheduleEntry>,
}

/// The rows Account delete checks, and the one write that removes an Account.
pub trait AccountDeletion {
    /// Account `id`, which must exist to be deleted.
    fn account(&self, id: AccountId) -> Option<Account>;

    /// How many Bill Plans pay from Account `id`. While any does it can't be deleted: a Plan has
    /// to pay from an Account.
    fn bill_plans_paying_from(&self, id: AccountId) -> usize;

    /// The Transactions booked to Account `id`.
    fn transactions_booked_to(&self, id: AccountId) -> Vec<TransactionId>;

    /// Every Bill Schedule entry, from which a delete takes those its Transactions paid.
    fn bill_entries(&self) -> Vec<BillScheduleEntry>;

    /// Writes `removal` as one write: the entries Unmatched, the Transactions and their Splits
    /// removed, and the Account removed, with its Balance Checks. All land or none do.
    fn remove_account(&mut self, removal: AccountRemoval);
}

/// The deletion port as the Delete Dialog holds it, shared with the Client that commits its
/// writes. Single-threaded, as both Clients' UI threads are.
pub type SharedDeletion = Rc<RefCell<dyn AccountDeletion>>;

/// What the Delete Dialog quotes beside the port's reads, read once when it opens: what names the
/// Account (`lib-accounts`' [`AccountUsage`]), and the code of the Unit its balance is in.
pub trait DeletionReads: AccountUsage {
    fn unit_code(&self, unit: UnitId) -> Option<String>;
}

/// What a confirmed delete removed, for the Client the Dialog closes to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountDeleted {
    pub id: AccountId,
    pub name: String,
    /// How many Transactions went with it.
    pub transactions: usize,
    /// How many Bill Schedule entries they paid, now Unmatched.
    pub unmatched: usize,
}

/// Navigation out of the Delete Dialog's Presenter, implemented by each Client and passed into
/// each `handle` call. A Toast or status-line message goes to the Shell through [`Notify`]
/// (ADR-0048).
pub trait AccountDeletionRouter: Notify {
    /// Closes the Delete Dialog. `outcome` is what its confirm removed, `None` when it was
    /// cancelled.
    fn close_dialog(&mut self, outcome: Option<AccountDeleted>);
}
