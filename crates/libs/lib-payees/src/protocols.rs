//! The ports Payees reach storage through, in the shape of `lib-units`'. They are synchronous
//! (ADR-0046): an adapter serves reads from memory and queues each write, so a write never fails
//! here; a failed save is the adapter's to report. `fake` implements them in memory.
//!
//! Until the Payees Presenters land (#832) the Desktop still writes Payees through `lib-ledger`'s
//! commands, which answer [`PayeeUsage`] from the Ledger; the repository is what `lib-database`'s
//! adapter will implement (#831).

use lib_core::PayeeId;

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
    fn payee_usage(&self, id: PayeeId) -> PayeeUse;
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

/// The ports borrowed together for one write: the delete reads usage while it writes the Payees.
pub struct PortsMut<'a> {
    pub payees: &'a mut dyn PayeeRepository,
    pub usage: &'a dyn PayeeUsage,
}
