//! The ports the Bills domain is stored and reads through. They are synchronous (ADR-0046): an
//! adapter serves reads from memory and queues each write, so a write never fails here; a failed
//! save is the adapter's to report. `fake` implements each in memory.
//!
//! Ports only, for now: the Bills View and its writers are still `lib-ledger`'s `bill_commands`,
//! which reach the repository ports through the Desktop's forwarding shim (#774). The read ports
//! wait for the Interactors that read through them.

use chrono::NaiveDate;
use lib_accounts::Account;
use lib_categories::Category;
use lib_core::{BillPlanId, TransactionId};
use lib_payees::Payee;
use lib_transactions::Transaction;

use crate::{BillPlan, BillScheduleEntry, EntryId};

/// The Bill Plans on the Ledger.
pub trait BillPlanRepository {
    /// Every Bill Plan, in the order first written.
    fn bill_plans(&self) -> Vec<BillPlan>;

    fn bill_plan(&self, id: BillPlanId) -> Option<BillPlan> {
        self.bill_plans().into_iter().find(|plan| plan.id == id)
    }

    /// Writes `plan`, replacing the row with its id or adding it. A Bill Plan is never deleted:
    /// one no longer wanted is deactivated.
    fn upsert_bill_plan(&mut self, plan: BillPlan);
}

/// The Bill Schedule: every entry generation has made, superseded ones included.
pub trait BillScheduleRepository {
    /// Every entry, in no particular order.
    fn entries(&self) -> Vec<BillScheduleEntry>;

    /// The entry for a Bill Plan and due date, the key generation looks an entry up by before it
    /// adds one (ADR-0019 as amended).
    fn entry(&self, id: EntryId) -> Option<BillScheduleEntry> {
        self.entries().into_iter().find(|entry| entry.id() == id)
    }

    /// Writes `entry`, replacing the one for its Bill Plan and due date or adding it. A replaced
    /// entry keeps its stored `row_id`, so a Bill Plan and due date only ever have one row. A Paid
    /// entry's Split must be stored, since the link is the Split's. An entry is never deleted:
    /// one an edit replaces is superseded.
    fn upsert_entry(&mut self, entry: BillScheduleEntry);
}

/// The Transactions Bills reads: Match candidates, and a Paid entry's amount and date.
pub trait BillTransactions {
    /// The Transactions dated from `from` to `to`, both included, in no particular order.
    fn transactions_between(&self, from: NaiveDate, to: NaiveDate) -> Vec<Transaction>;

    /// The Transaction a Paid entry's Split belongs to.
    fn paid_transaction(&self, id: TransactionId) -> Option<Transaction>;
}

/// The Accounts a Bill Plan may be paid from.
pub trait BillAccounts {
    fn bill_accounts(&self) -> Vec<Account>;
}

/// The Categories a Bill Plan may be filed under.
pub trait BillCategories {
    fn bill_categories(&self) -> Vec<Category>;
}

/// The Payees a Bill Plan may be paid to.
pub trait BillPayees {
    fn bill_payees(&self) -> Vec<Payee>;
}
