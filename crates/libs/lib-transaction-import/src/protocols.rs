//! The port an Import's commit writes through, and the reads the step is drawn from.
//!
//! [`TransactionImport`] is the module's one atomic port: an Import is its new Payees, their
//! aliases and its Transactions, written together or not at all. It is synchronous (ADR-0046): an
//! adapter checks the plan against the rows it holds in memory and queues every write in the batch
//! every adapter shares, so the bin's one commit after the call lands them as one transaction, and
//! a failed save is the adapter's to report. `fake` implements it in memory.
//!
//! The reads name the other domains' rows as Import-owned values, or `lib-payees`' own, so the
//! module depends on no feature crate but the two it writes. `lib-database`'s adapters answer them
//! from the rows they hold.

use std::cell::RefCell;
use std::rc::Rc;

use lib_core::AccountId;
use lib_payees::{Payee, PayeeCategory};
use lib_toast::Notify;

use crate::entity::Statement;
use crate::error::Result;
use crate::plan::{ImportCommitted, ImportPlan};

/// The one write an Import makes.
pub trait TransactionImport {
    /// Writes `plan`'s new Payees with their aliases and its Transactions with their Splits, as one
    /// write: all land or none do. Refused, writing nothing, when [`ImportPlan::check`] refuses it
    /// against the stored Payees.
    fn commit(&mut self, plan: ImportPlan) -> Result<ImportCommitted>;
}

/// The import port as the step holds it, shared with the Client that commits its writes.
/// Single-threaded, as both Clients' UI threads are.
pub type SharedImport = Rc<RefCell<dyn TransactionImport>>;

/// Where a statement comes from: its raw lines and the Account they belong to. The seeded
/// statement is the only source today (`statement`).
pub trait StatementSource {
    fn statement(&self) -> Statement;
}

/// An Account as an Import reads it: the one a statement books to, which must be active to take
/// Transactions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportAccount {
    pub id: AccountId,
    pub name: String,
    pub is_active: bool,
}

/// The Account a statement belongs to. A read port, so the module needn't depend on the Accounts
/// crate.
pub trait ImportAccounts {
    /// Account `id`, or `None` when it is gone.
    fn import_account(&self, id: AccountId) -> Option<ImportAccount>;
}

/// The Payees a statement is matched against: every Payee with its aliases, inactive ones too.
/// Only active Payees match or are offered, but an inactive one's name stays taken.
pub trait ImportPayees {
    fn import_payees(&self) -> Vec<Payee>;
}

/// The Categories a row may be filed under: every leaf, in the Categories' order, labelled as
/// `lib-payees`' `PayeeCategories` labels them, which the Categories adapter already answers.
pub trait ImportCategories {
    fn import_categories(&self) -> Vec<PayeeCategory>;
}

/// Everything the Import step reads and writes, as the Client builds it once. The payees read and
/// the write are separate handles so a Client may back them with one store (an `Rc<RefCell<_>>`
/// coerces to both), as the in-memory fake does.
#[derive(Clone)]
pub struct ImportPorts {
    pub import: SharedImport,
    pub payees: Rc<RefCell<dyn ImportPayees>>,
    pub accounts: Rc<dyn ImportAccounts>,
    pub categories: Rc<dyn ImportCategories>,
    pub statement: Rc<dyn StatementSource>,
}

/// Navigation out of the Import step's Presenter, implemented by each Client and passed into each
/// `handle` call. A Toast or status-line message goes to the Shell through [`Notify`] (ADR-0048).
pub trait ImportRouter: Notify {
    /// Leaves the step. `committed` is what **continue** wrote, `None` when it was cancelled.
    fn close_import(&mut self, committed: Option<ImportCommitted>);
}
