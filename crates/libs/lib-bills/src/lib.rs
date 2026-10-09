//! Pure Bills domain rules (`docs/bills.md`, ADR-0019 and its amendments), `gpui`-free and I/O-free,
//! shared by the Desktop and the TUI.
//!
//! The rules are the Desktop Bills Surface map's settled decisions (#365–#368):
//!
//! - A [`BillPlan`]'s due dates step from its First Due: 7 or 14 days for Weekly and Fortnightly,
//!   and for Monthly, Quarterly and Annually its day of the month, clamped to a shorter month's last
//!   day but always computed from the anchor (31 Jan gives 28 Feb, then 31 Mar). Ends On is
//!   inclusive.
//! - A [`BillScheduleEntry`] is identified by its Bill Plan and due date ([`EntryId`]), the stand-in
//!   for ADR-0019's deterministic UUIDv5, so regenerating a date finds the same row.
//! - Generation ([`populate`]) runs through the end of next calendar month and always keeps at
//!   least one unresolved entry due today or later per active Bill Plan. It skips a due date whose
//!   Recurrence period already holds a Paid or Skipped entry, and clears `superseded` on a row an
//!   edit brings back rather than adding a second one.
//! - An edit to Recurrence, First Due or Ends On, or deactivating, supersedes (never deletes) the
//!   unresolved entries due today or later; Overdue, Paid and Skipped entries are never touched.
//! - Upcoming, Due and Overdue are derived from `today` by calendar month; Needs Attention is a date
//!   rule (unresolved and due on or before today plus the Attention Lead).
//!
//! Matching an entry to a Transaction Split, and everything that reads Accounts, Categories or
//! Payees, stays with the Client that owns those records.

pub mod bills;

pub use bills::*;
pub mod history;
pub use history::*;

use chrono::{Datelike, Duration, Months, NaiveDate};
use lib_core::Money;

/// How often a Bill Plan falls due.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recurrence {
    Weekly,
    Fortnightly,
    Monthly,
    Quarterly,
    Annually,
    /// A single, non-repeating due date: its First Due.
    OneShot,
}

impl Recurrence {
    pub const ALL: [Recurrence; 6] = [
        Recurrence::Weekly,
        Recurrence::Fortnightly,
        Recurrence::Monthly,
        Recurrence::Quarterly,
        Recurrence::Annually,
        Recurrence::OneShot,
    ];

    /// The `n`th due date stepped from `first_due`, or `None` past a One-shot's only date (or past
    /// the end of the calendar).
    pub fn occurrence(self, first_due: NaiveDate, n: u32) -> Option<NaiveDate> {
        let months = |step: u32| first_due.checked_add_months(Months::new(n.checked_mul(step)?));
        match self {
            Recurrence::Weekly => first_due.checked_add_signed(Duration::days(7 * i64::from(n))),
            Recurrence::Fortnightly => {
                first_due.checked_add_signed(Duration::days(14 * i64::from(n)))
            }
            Recurrence::Monthly => months(1),
            Recurrence::Quarterly => months(3),
            Recurrence::Annually => months(12),
            Recurrence::OneShot => (n == 0).then_some(first_due),
        }
    }

    /// Whether two due dates fall in the same Recurrence period: the same 7- or 14-day step, or
    /// the same calendar month, quarter or year. A One-shot has only the one period.
    pub fn same_period(self, a: NaiveDate, b: NaiveDate) -> bool {
        let quarter = |d: NaiveDate| (d.year(), d.month0() / 3);
        match self {
            Recurrence::Weekly => (a - b).num_days().abs() < 7,
            Recurrence::Fortnightly => (a - b).num_days().abs() < 14,
            Recurrence::Monthly => (a.year(), a.month()) == (b.year(), b.month()),
            Recurrence::Quarterly => quarter(a) == quarter(b),
            Recurrence::Annually => a.year() == b.year(),
            Recurrence::OneShot => true,
        }
    }
}

/// Whether a Bill Plan's Planned Amount is a promise (Netflix) or a planning figure (electricity).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmountKind {
    Fixed,
    Estimated,
}

/// The recurring definition of an expected payment obligation (glossary: Bill Plan).
#[derive(Debug, Clone, PartialEq)]
pub struct BillPlan {
    pub id: u32,
    pub name: String,
    /// A leaf Expense Category id.
    pub category_id: u32,
    /// The Unit code (e.g. `"aud"`), always the Account's; locked after creation.
    pub unit: String,
    /// The Account id it's paid from; one that takes Transactions.
    pub account_id: u32,
    pub payee_id: Option<u32>,
    /// Positive: the magnitude of the expected payment.
    pub planned_amount: Money,
    pub amount_kind: AmountKind,
    pub recurrence: Recurrence,
    pub first_due: NaiveDate,
    /// Inclusive, never before `first_due`, and always `None` for a One-shot.
    pub ends_on: Option<NaiveDate>,
    /// Days before a due date an unresolved entry enters Needs Attention; `None` counts as zero.
    pub attention_lead: Option<u32>,
    pub is_active: bool,
    /// The earliest due date generation may create. `first_due` on creation, then today whenever
    /// an edit or a reactivation regenerates, so neither backfills due dates already past.
    pub generate_from: NaiveDate,
}

/// A Bill Schedule entry's identity: its Bill Plan and due date (ADR-0019's `(bill_plan_id,
/// due_date)` key), so every generation of the same date finds the same row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EntryId {
    pub plan_id: u32,
    pub due: NaiveDate,
}

/// One Split of one Transaction: what a Bill Schedule entry is Matched to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SplitRef {
    pub transaction_id: u32,
    pub split_index: usize,
}

/// How a Bill Schedule entry was resolved, if it has been.
#[derive(Debug, Clone, PartialEq)]
pub enum Resolution {
    /// Upcoming, Due or Overdue, derived from its due date; reads its figures live from its Plan.
    Unresolved,
    /// Matched to one Split, whose amount and Transaction date take precedence over the Plan's.
    Paid(SplitRef),
    /// Deliberately not paid, keeping the Planned Amount at the moment it was skipped.
    Skipped { planned: Money },
}

/// One dated instance of a Bill Plan (glossary: Bill Schedule).
#[derive(Debug, Clone, PartialEq)]
pub struct BillScheduleEntry {
    pub plan_id: u32,
    pub due: NaiveDate,
    pub resolution: Resolution,
    /// Replaced by a Bill Plan edit or deactivation: kept, but ignored by every view and figure.
    pub superseded: bool,
}

impl BillScheduleEntry {
    pub fn id(&self) -> EntryId {
        EntryId {
            plan_id: self.plan_id,
            due: self.due,
        }
    }

    pub fn is_unresolved(&self) -> bool {
        self.resolution == Resolution::Unresolved
    }

    /// Unresolved and not superseded: the entries Pay, Match and Skip act on.
    pub fn is_open(&self) -> bool {
        self.is_unresolved() && !self.superseded
    }
}

/// A Bill Schedule entry's status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BillStatus {
    /// Due in a later calendar month.
    Upcoming,
    /// Due this calendar month, today or later.
    Due,
    /// Due before today, still unresolved.
    Overdue,
    Paid,
    Skipped,
}

/// The status an entry shows on `today`.
pub fn status(entry: &BillScheduleEntry, today: NaiveDate) -> BillStatus {
    match entry.resolution {
        Resolution::Paid(_) => BillStatus::Paid,
        Resolution::Skipped { .. } => BillStatus::Skipped,
        Resolution::Unresolved if entry.due < today => BillStatus::Overdue,
        Resolution::Unresolved if month_of(entry.due) > month_of(today) => BillStatus::Upcoming,
        Resolution::Unresolved => BillStatus::Due,
    }
}

/// Whether an entry is in Needs Attention on `today`: open and due on or before today plus its
/// Plan's Attention Lead, whatever its status and across month boundaries.
pub fn needs_attention(entry: &BillScheduleEntry, plan: &BillPlan, today: NaiveDate) -> bool {
    let lead = Duration::days(i64::from(plan.attention_lead.unwrap_or(0)));
    entry.is_open() && entry.due <= today + lead
}

/// The entries in Needs Attention, by due date: the Dashboard's Bill rows, and (counted) the Bills
/// rail badge, so both read the one rule.
pub fn attention_entries(
    plans: &[BillPlan],
    entries: &[BillScheduleEntry],
    today: NaiveDate,
) -> Vec<EntryId> {
    let mut ids: Vec<EntryId> = entries
        .iter()
        .filter(|entry| get(plans, entry.plan_id).is_some_and(|p| needs_attention(entry, p, today)))
        .map(BillScheduleEntry::id)
        .collect();
    ids.sort_by_key(|id| (id.due, id.plan_id));
    ids
}

pub fn get(plans: &[BillPlan], id: u32) -> Option<&BillPlan> {
    plans.iter().find(|plan| plan.id == id)
}

pub fn entry(entries: &[BillScheduleEntry], id: EntryId) -> Option<&BillScheduleEntry> {
    entries.iter().find(|entry| entry.id() == id)
}

pub fn entry_mut(entries: &mut [BillScheduleEntry], id: EntryId) -> Option<&mut BillScheduleEntry> {
    entries.iter_mut().find(|entry| entry.id() == id)
}

/// The last date generation reaches: the end of next calendar month.
pub fn horizon(today: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
        .and_then(|first| first.checked_add_months(Months::new(2)))
        .and_then(|next| next.pred_opt())
        .unwrap_or(NaiveDate::MAX)
}

/// A calendar month as an ordered `(year, month)` pair, so months compare without a date range.
fn month_of(date: NaiveDate) -> (i32, u32) {
    (date.year(), date.month())
}

/// Brings `plan`'s entries up to the horizon: adds missing due dates, revives superseded rows an
/// edit brought back, and skips dates whose Recurrence period is already settled. Always leaves
/// at least one open entry due today or later while the Plan has one before its Ends On. Does
/// nothing for an inactive Plan.
fn populate_plan(plan: &BillPlan, entries: &mut Vec<BillScheduleEntry>, today: NaiveDate) {
    if !plan.is_active {
        return;
    }
    let horizon = horizon(today);
    let ends_on = plan.ends_on.unwrap_or(NaiveDate::MAX);
    let mut n = 0;
    while let Some(due) = plan.recurrence.occurrence(plan.first_due, n) {
        n += 1;
        if due > ends_on {
            break;
        }
        let past_horizon = due > horizon;
        if past_horizon
            && entries
                .iter()
                .any(|e| e.plan_id == plan.id && e.is_open() && e.due >= today)
        {
            break;
        }
        if due < plan.generate_from {
            continue;
        }
        place(plan, entries, due);
        if past_horizon {
            break;
        }
    }
}

/// Adds (or revives) the entry for `due` unless its Recurrence period is already settled.
fn place(plan: &BillPlan, entries: &mut Vec<BillScheduleEntry>, due: NaiveDate) {
    let settled = entries.iter().any(|e| {
        e.plan_id == plan.id
            && e.due != due
            && !e.superseded
            && !e.is_unresolved()
            && plan.recurrence.same_period(e.due, due)
    });
    let id = EntryId {
        plan_id: plan.id,
        due,
    };
    match entry_mut(entries, id) {
        Some(existing) => {
            if !settled {
                existing.superseded = false;
            }
        }
        None if !settled => entries.push(BillScheduleEntry {
            plan_id: plan.id,
            due,
            resolution: Resolution::Unresolved,
            superseded: false,
        }),
        None => {}
    }
}

/// Runs generation for every Bill Plan: on Client start and when a running Client crosses into a
/// new calendar month.
pub fn populate(plans: &[BillPlan], entries: &mut Vec<BillScheduleEntry>, today: NaiveDate) {
    for plan in plans {
        populate_plan(plan, entries, today);
    }
    entries.sort_by_key(|entry| (entry.due, entry.plan_id));
}

/// Marks `plan_id`'s unresolved entries due today or later as superseded.
pub fn supersede_from(entries: &mut [BillScheduleEntry], plan_id: u32, today: NaiveDate) {
    for entry in entries
        .iter_mut()
        .filter(|e| e.plan_id == plan_id && e.is_unresolved() && e.due >= today)
    {
        entry.superseded = true;
    }
}

/// The Bill Plans and their Bill Schedule, held together because every write to one can touch
/// the other (an edit supersedes entries, a new Plan populates them). Mutations go through
/// [`BillService::edit`] so every write is one closure a store can notify around.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BillService {
    plans: Vec<BillPlan>,
    entries: Vec<BillScheduleEntry>,
}

impl BillService {
    pub fn from_rows(plans: Vec<BillPlan>, entries: Vec<BillScheduleEntry>) -> Self {
        Self { plans, entries }
    }

    pub fn plans(&self) -> &[BillPlan] {
        &self.plans
    }

    pub fn entries(&self) -> &[BillScheduleEntry] {
        &self.entries
    }

    /// Edits the Plans and entries in one closure, so a caller can make a multi-step change (an
    /// insert that populates, a pay that settles) without a second borrow.
    pub fn edit<R>(
        &mut self,
        change: impl FnOnce(&mut Vec<BillPlan>, &mut Vec<BillScheduleEntry>) -> R,
    ) -> R {
        change(&mut self.plans, &mut self.entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn today() -> NaiveDate {
        date(2026, 9, 19)
    }

    fn open(due: NaiveDate) -> BillScheduleEntry {
        BillScheduleEntry {
            plan_id: 1,
            due,
            resolution: Resolution::Unresolved,
            superseded: false,
        }
    }

    #[test]
    fn service_edit_changes_plans_and_entries_together() {
        let mut service = BillService::from_rows(Vec::new(), vec![open(date(2026, 9, 20))]);
        let superseded = service.edit(|_plans, entries| {
            supersede_from(entries, 1, today());
            entries.iter().filter(|e| e.superseded).count()
        });
        assert_eq!(superseded, 1);
        assert!(service.plans().is_empty());
        assert!(service.entries()[0].superseded);
    }

    #[test]
    fn monthly_clamps_to_month_end_but_steps_from_the_anchor() {
        let r = Recurrence::Monthly;
        let anchor = date(2026, 1, 31);
        assert_eq!(r.occurrence(anchor, 1), Some(date(2026, 2, 28)));
        assert_eq!(r.occurrence(anchor, 2), Some(date(2026, 3, 31)));
        assert_eq!(r.occurrence(date(2028, 1, 31), 1), Some(date(2028, 2, 29)));
    }

    #[test]
    fn weekly_fortnightly_quarterly_annually_and_one_shot_step() {
        let anchor = date(2026, 9, 1);
        assert_eq!(
            Recurrence::Weekly.occurrence(anchor, 2),
            Some(date(2026, 9, 15))
        );
        assert_eq!(
            Recurrence::Fortnightly.occurrence(anchor, 2),
            Some(date(2026, 9, 29))
        );
        assert_eq!(
            Recurrence::Quarterly.occurrence(anchor, 1),
            Some(date(2026, 12, 1))
        );
        assert_eq!(
            Recurrence::Annually.occurrence(anchor, 1),
            Some(date(2027, 9, 1))
        );
        assert_eq!(Recurrence::OneShot.occurrence(anchor, 0), Some(anchor));
        assert_eq!(Recurrence::OneShot.occurrence(anchor, 1), None);
    }

    #[test]
    fn same_period_is_the_step_or_the_calendar_unit() {
        assert!(Recurrence::Weekly.same_period(date(2026, 9, 1), date(2026, 9, 7)));
        assert!(!Recurrence::Weekly.same_period(date(2026, 9, 1), date(2026, 9, 8)));
        assert!(Recurrence::Monthly.same_period(date(2026, 9, 1), date(2026, 9, 30)));
        assert!(Recurrence::Quarterly.same_period(date(2026, 7, 1), date(2026, 9, 30)));
        assert!(!Recurrence::Quarterly.same_period(date(2026, 9, 30), date(2026, 10, 1)));
        assert!(Recurrence::Annually.same_period(date(2026, 1, 1), date(2026, 12, 31)));
    }

    #[test]
    fn horizon_is_the_end_of_next_month_across_a_year_end() {
        assert_eq!(horizon(today()), date(2026, 10, 31));
        assert_eq!(horizon(date(2026, 12, 5)), date(2027, 1, 31));
    }

    #[test]
    fn statuses_are_calendar_month() {
        assert_eq!(
            status(&open(date(2026, 9, 18)), today()),
            BillStatus::Overdue
        );
        assert_eq!(
            status(&open(date(2026, 8, 30)), today()),
            BillStatus::Overdue
        );
        assert_eq!(status(&open(today()), today()), BillStatus::Due);
        assert_eq!(status(&open(date(2026, 9, 30)), today()), BillStatus::Due);
        assert_eq!(
            status(&open(date(2026, 10, 1)), today()),
            BillStatus::Upcoming
        );
    }
}
