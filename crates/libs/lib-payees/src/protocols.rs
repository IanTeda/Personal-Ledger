//! The ports Payees reach storage and the other domains through, in the shape of `lib-units`'.
//! They are synchronous (ADR-0046): an adapter serves reads from memory and queues each write, so a
//! write never fails here; a failed save is the adapter's to report. `fake` implements them in
//! memory.
//!
//! `lib-database`'s Payees adapter implements the repository and usage (#831). The Presenters
//! reach every port together, through [`PayeesPorts`], and ask for navigation through
//! [`PayeesRouter`], which each Client implements.

use std::cell::RefCell;
use std::rc::Rc;

use lib_core::{CategoryId, PayeeId};
use lib_toast::Notify;

use crate::Payee;

/// The Payees on the Ledger, each with its Payee Aliases.
pub trait PayeeRepository {
    /// Every Payee, in no particular order.
    fn payees(&self) -> Vec<Payee>;

    fn payee(&self, id: PayeeId) -> Option<Payee> {
        self.payees().into_iter().find(|payee| payee.id == id)
    }

    /// Writes `payee` with its aliases, replacing the row with its id or adding it.
    fn upsert_payee(&mut self, payee: Payee);

    /// Removes Payee `id` and its aliases.
    fn delete_payee(&mut self, id: PayeeId);
}

/// How many rows in other domains name a Payee. A read port, so `lib-payees` needn't depend on the
/// Transactions or Bills crates to refuse deleting a Payee in use.
pub trait PayeeUsage {
    /// `None` when the count can't be read: unknown, never taken as unused, so a Payee that may be
    /// in use is never offered for deleting (#840).
    fn payee_usage(&self, id: PayeeId) -> Option<PayeeUse>;
}

/// How many rows name a Payee, carried by the delete refusal so a Dialog can say why and offer
/// Deactivate instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PayeeUse {
    pub splits: usize,
    pub bill_plans: usize,
}

impl PayeeUse {
    pub fn is_used(self) -> bool {
        self.splits + self.bill_plans > 0
    }
}

/// A Category a Payee may default to, as Payees reads it: a leaf, with the label the Default
/// category select shows. The label names exactly one Category, so a leaf sharing its name with
/// another is labelled under its parent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayeeCategory {
    pub id: CategoryId,
    pub label: String,
}

/// The Categories a Payee may default to: every leaf, in the Categories' order. A read port, so
/// `lib-payees` needn't depend on the Categories crate.
pub trait PayeeCategories {
    fn payee_categories(&self) -> Vec<PayeeCategory>;
}

/// The ports borrowed together for one write: the delete reads usage while it writes the Payees.
pub struct PortsMut<'a> {
    pub payees: &'a mut dyn PayeeRepository,
    pub usage: &'a dyn PayeeUsage,
}

/// Every Payees port, as one value the Presenters share.
pub trait PayeesPorts {
    fn payees(&self) -> &dyn PayeeRepository;

    fn usage(&self) -> &dyn PayeeUsage;

    fn categories(&self) -> &dyn PayeeCategories;

    fn split_mut(&mut self) -> PortsMut<'_>;
}

/// The ports as the page and its Dialogs hold them: one set, shared, so a Payee a Dialog adds is
/// on the page as soon as the Dialog closes. Single-threaded, as both Clients' UI threads are.
pub type SharedPorts = Rc<RefCell<dyn PayeesPorts>>;

/// A Payees Dialog, as the page asks the Router to open it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayeesDialog {
    AddPayee,
    EditPayee(PayeeId),
    /// Delete, Deactivate or Reactivate, whichever the Payee's use calls for when it opens.
    DeletePayee(PayeeId),
}

/// What a Dialog did to a Payee.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayeeChange {
    /// Added or edited.
    Saved,
    Deleted,
    Deactivated,
    Reactivated,
}

/// What a confirmed Payees Dialog did, handed to the page when the Dialog closes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayeesOutcome {
    pub change: PayeeChange,
    pub id: PayeeId,
    /// The Payee's name as written, read before a delete removes it.
    pub name: String,
}

/// Navigation out of the Payees Presenters, implemented by each Client. The Client passes it into
/// each `handle` call rather than the Presenter holding it, since the Desktop can only open a
/// Dialog with the gpui context that call runs in. A Toast or status-line message goes to the
/// Shell through [`Notify`] (ADR-0048).
pub trait PayeesRouter: Notify {
    /// Builds `dialog`'s Presenter on the shared ports and hosts its View. Returns whether it
    /// opened: a Dialog on a Payee removed since it was asked for has no Presenter to build.
    fn open_dialog(&mut self, dialog: PayeesDialog) -> bool;

    /// Shows the Transactions of the Payee `id`: the Transactions page filtered to it, from its
    /// first row.
    fn open_transactions(&mut self, id: PayeeId);

    /// Closes the open Payees Dialog. `outcome` is what its confirm did, `None` when it was
    /// cancelled; the Client hands it to the page as [`PayeesPageEvent::DialogClosed`].
    ///
    /// [`PayeesPageEvent::DialogClosed`]: crate::presenter::PayeesPageEvent::DialogClosed
    fn close_dialog(&mut self, outcome: Option<PayeesOutcome>);
}
