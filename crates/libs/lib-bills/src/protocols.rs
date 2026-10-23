//! The ports the Bills domain is stored and reads through. They are synchronous (ADR-0046): an
//! adapter serves reads from memory and queues each write, so a write never fails here; a failed
//! save is the adapter's to report. `fake` implements each in memory.
//!
//! The Presenters reach the ports together, through [`BillsPorts`], and ask for navigation through
//! [`BillsRouter`], which each Client implements.

use std::cell::RefCell;
use std::rc::Rc;

use chrono::NaiveDate;
use lib_core::{AccountId, BillPlanId, CategoryId, PayeeId, TransactionId, UnitId};
use lib_toast::ToastKind;
use lib_transactions::Transaction;

use crate::{BillPlan, BillScheduleEntry, EntryId, SplitRef};

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

/// Both Bills repositories as one, since a Bills write can touch both: an edit saves the Plan and
/// supersedes its entries. Every type holding both has it.
pub trait BillsRepository: BillPlanRepository + BillScheduleRepository {}

impl<T: BillPlanRepository + BillScheduleRepository + ?Sized> BillsRepository for T {}

/// The Transactions Bills reads: Match candidates, and a Paid entry's amount and date.
pub trait BillTransactions {
    /// The Transactions dated from `from` to `to`, both included, in no particular order.
    fn transactions_between(&self, from: NaiveDate, to: NaiveDate) -> Vec<Transaction>;

    /// The Transaction a Paid entry's Split belongs to.
    fn paid_transaction(&self, id: TransactionId) -> Option<Transaction>;
}

/// An Account as Bills reads it. `takes_transactions` is the Accounts' rule, filled by the side
/// that answers [`BillAccounts`], so Bills checks the flag rather than knowing Account types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillAccount {
    pub id: AccountId,
    pub name: String,
    pub unit: UnitId,
    pub takes_transactions: bool,
}

/// A Category as Bills reads it. `path` (ancestors then `name`) and `is_leaf` are the
/// Categories' rules, filled by the side that answers [`BillCategories`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillCategory {
    pub id: CategoryId,
    pub name: String,
    pub path: String,
    pub is_expense: bool,
    pub is_leaf: bool,
}

/// A Payee as Bills reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillPayee {
    pub id: PayeeId,
    pub name: String,
    pub is_active: bool,
}

/// The Accounts a Bill Plan may be paid from: every Account, flagged, not a filtered list.
pub trait BillAccounts {
    fn bill_accounts(&self) -> Vec<BillAccount>;
}

/// The Categories a Bill Plan may be filed under: every Category, flagged.
pub trait BillCategories {
    fn bill_categories(&self) -> Vec<BillCategory>;
}

/// The Payees a Bill Plan may be paid to: every Payee, inactive ones included.
pub trait BillPayees {
    fn bill_payees(&self) -> Vec<BillPayee>;
}

/// The Units Bills names its amounts in. A read port, so `lib-bills` needn't depend on the Units
/// crate to write `aud`.
pub trait BillUnits {
    /// Unit `id`'s code, or `None` when it is gone.
    fn unit_code(&self, id: UnitId) -> Option<String>;

    /// The base Unit, whose amounts show no code.
    fn base_unit(&self) -> Option<UnitId>;
}

/// Every Bills port, as one value the Presenters share. An Interactor writes the Bills while
/// reading the other domains, so [`Self::split_mut`] lends them out together.
pub trait BillsPorts {
    fn bills(&self) -> &dyn BillsRepository;

    fn transactions(&self) -> &dyn BillTransactions;

    fn accounts(&self) -> &dyn BillAccounts;

    fn categories(&self) -> &dyn BillCategories;

    fn payees(&self) -> &dyn BillPayees;

    fn units(&self) -> &dyn BillUnits;

    fn split_mut(&mut self) -> PortsMut<'_>;
}

/// The ports borrowed together for one Interactor call.
pub struct PortsMut<'a> {
    pub bills: &'a mut dyn BillsRepository,
    pub transactions: &'a dyn BillTransactions,
    pub accounts: &'a dyn BillAccounts,
    pub categories: &'a dyn BillCategories,
}

/// The ports as the page and its Dialogs hold them: one set, shared, so a Plan a Dialog adds is in
/// the Planner as soon as the Dialog closes. Single-threaded, as both Clients' UI threads are.
pub type SharedPorts = Rc<RefCell<dyn BillsPorts>>;

/// What a confirmed Bills Dialog did, handed to the page when the Dialog closes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillsOutcome {
    /// Added or edited: the Plan the Planner selects.
    PlanSaved(BillPlanId),
    Skipped,
    /// The entry resolved or was superseded underneath the Skip Dialog, which has no form to show
    /// the refusal on, so it closes and the page says why.
    SkipRefused,
}

/// Pay on a Schedule entry, as the page asks the Router to open it: the entry, its Match
/// candidates and the one the Match list pre-selects, read when Pay was pressed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayRequest {
    pub entry: EntryId,
    pub candidates: Vec<SplitRef>,
    pub preselected: Option<SplitRef>,
}

/// Navigation out of the Bills Presenters, implemented by each Client. The Client passes it into
/// each `handle` call rather than the Presenter holding it, since the Desktop can only open a
/// Dialog with the gpui context that call runs in.
pub trait BillsRouter {
    /// Builds the Add bill plan form (`None`) or Bill Plan `id`'s Edit form on the shared ports and
    /// hosts its View. Returns whether it opened: a Plan removed since has no form to build.
    fn open_plan_form(&mut self, plan: Option<BillPlanId>) -> bool;

    /// Builds the Skip Dialog on entry `entry` and hosts its View. Returns whether it opened.
    fn open_skip(&mut self, entry: EntryId) -> bool;

    /// Opens the Pay Dialog on `request`'s entry. Returns whether it opened.
    fn open_pay(&mut self, request: PayRequest) -> bool;

    /// Closes the open Bills Dialog. `outcome` is what its confirm did, `None` when it was
    /// cancelled; the Client hands it to the page as [`BillsPageEvent::DialogClosed`].
    ///
    /// [`BillsPageEvent::DialogClosed`]: crate::presenter::BillsPageEvent::DialogClosed
    fn close_dialog(&mut self, outcome: Option<BillsOutcome>);

    /// Shows Transaction `id` in the Transactions module.
    fn show_transaction(&mut self, id: TransactionId);

    /// Raises a Toast in the Shell.
    fn raise_toast(&mut self, kind: ToastKind, text: String);

    /// Puts `text` on the Shell's status line, or clears it.
    fn set_status(&mut self, text: Option<String>);
}
