//! The ports the Dashboard reads the other domains through, each returning `lib-dashboard`'s own
//! rows, so the crate depends on no feature crate. The rules stay with their domains: which Bill
//! Schedule entries need attention is `lib-bills`' `attention_entries`, and the default Budget's
//! figures and bars are `lib-budgets`'. An adapter answers each port with those rules' results, so
//! nothing here re-derives them. Both are synchronous, served from memory (ADR-0046).
//!
//! The Dashboard asks for navigation through [`DashboardRouter`], which each Client implements.

use chrono::NaiveDate;
use lib_core::{BillPlanId, Money, Period};
use lib_toast::Notify;

/// A Bill Schedule entry's identity, its Bill Plan and due date, as `lib-bills`' `EntryId` keys it
/// (ADR-0019).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DashboardEntryId {
    pub plan_id: BillPlanId,
    pub due: NaiveDate,
}

/// One Bill Schedule entry needing attention.
#[derive(Debug, Clone, PartialEq)]
pub struct DashboardBill {
    pub id: DashboardEntryId,
    /// Its Bill Plan's name.
    pub plan: String,
    /// Overdue as of the day asked about, rather than due.
    pub overdue: bool,
    /// What it is expected to cost, in its Plan's Unit.
    pub amount: Money,
    /// Its Plan's Unit code, empty once the Unit has gone.
    pub unit_code: String,
}

/// The Bill Schedule entries needing attention, and the base Unit their amounts are read against.
pub trait DashboardBills {
    /// The entries needing attention on `today`, in the order `lib-bills`' `attention_entries`
    /// gives them.
    fn attention_bills(&self, today: NaiveDate) -> Vec<DashboardBill>;

    /// The Ledger's base Unit's code, `None` when it has none.
    fn base_unit_code(&self) -> Option<String>;
}

/// How far through its month a day is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DashboardElapsed {
    /// The day of the month.
    pub day: u32,
    pub days_in_month: u32,
    /// Whole percent of the month gone.
    pub percent: u32,
}

/// One bar of the budget list: a budgeted top-level Category's spending this month.
#[derive(Debug, Clone, PartialEq)]
pub struct DashboardBudgetBar {
    /// The Category's name.
    pub category: String,
    pub spent: Money,
    pub budget: Money,
    /// Spent more than its budget.
    pub over: bool,
}

/// The default Budget's current month.
#[derive(Debug, Clone, PartialEq)]
pub struct DashboardBudgetMonth {
    pub month: Period,
    pub elapsed: DashboardElapsed,
    /// The fullest bars first, as `lib-budgets`' `dashboard_bars` gives them.
    pub bars: Vec<DashboardBudgetBar>,
}

/// The default Budget's figures.
pub trait DashboardBudgets {
    /// The default Budget's month containing `today`, `None` when no Budget is the default.
    fn default_budget_month(&self, today: NaiveDate) -> Option<DashboardBudgetMonth>;
}

/// Every Dashboard read port, as the one value the Presenter reads through.
pub trait DashboardReads: DashboardBills + DashboardBudgets {}

impl<T> DashboardReads for T where T: DashboardBills + DashboardBudgets + ?Sized {}

/// A command the no Ledger open content names as a click target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoLedgerCommand {
    Open,
    New,
}

impl NoLedgerCommand {
    /// The command's name, as the palette knows it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::New => "new",
        }
    }
}

/// Navigation out of the Dashboard, implemented by each Client and passed into each `handle`
/// call. A Toast or status-line message goes to the Shell through [`Notify`] (ADR-0048).
pub trait DashboardRouter: Notify {
    /// The Bills Schedule tab with entry `id` selected.
    fn reveal_bill_entry(&mut self, id: DashboardEntryId);

    /// Runs `command` as the palette would.
    fn run_no_ledger_command(&mut self, command: NoLedgerCommand);
}
