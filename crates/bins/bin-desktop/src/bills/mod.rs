//! Pure Bills-surface domain types and the stub dataset behind them (`docs/ux/desktop/12-bills/`,
//! `docs/bills.md`, ADR-0019 and its amendments) -- `gpui`-free and in-memory, the same "pure
//! state, chrome renders it" split `tags.rs` uses. Nothing here reads `lib_database`.
//!
//! The rules are the Desktop Bills Surface map's settled decisions (#365–#368):
//!
//! - A [`BillPlan`]'s due dates step from its First Due: 7 or 14 days for Weekly and Fortnightly,
//!   and for Monthly, Quarterly and Annually its day of the month, clamped to a shorter month's last
//!   day but always computed from the anchor (31 Jan gives 28 Feb, then 31 Mar). Ends On is
//!   inclusive.
//! - A [`BillScheduleEntry`] is identified by its Bill Plan and due date ([`EntryId`]), the stub's
//!   stand-in for ADR-0019's deterministic UUIDv5, so regenerating a date finds the same row.
//! - Generation ([`populate`]) runs through the end of next calendar month and always keeps at
//!   least one unresolved entry due today or later per active Bill Plan. It skips a due date whose
//!   Recurrence period already holds a Paid or Skipped entry, and clears `superseded` on a row an
//!   edit brings back rather than adding a second one.
//! - An edit to Recurrence, First Due or Ends On, or deactivating, supersedes (never deletes) the
//!   unresolved entries due today or later; Overdue, Paid and Skipped entries are never touched.
//! - Upcoming, Due and Overdue are derived from `today` by calendar month; Needs Attention is a date
//!   rule (unresolved and due on or before today plus the Attention Lead).
//! - Settlement is a Match to one Split, one-to-one. The stub records the Match on the entry as a
//!   [`SplitRef`] rather than as a `bill_schedule_id` on the Split, so the shared Transactions stub
//!   keeps its shape; a Transaction "carries a Bill" when any entry points at one of its Splits.
//!
//! Plan and Transaction ids are the stubs' `u32`s, matching every other desktop stub.

pub(crate) mod form;
pub(crate) mod history;
pub(crate) mod pay_form;

use bigdecimal::{BigDecimal, Signed};
use chrono::{Datelike, Duration, Months, NaiveDate};
use lib_core::{CategoryTypes, Money, TransactionStatus};

use crate::{
    accounts::Account,
    bills::form::BillPlanForm,
    bills::pay_form::PayForm,
    categories::{self, Category},
    dialog_host::{Dialog, DialogKey, DialogOutcome},
    field::TextField,
    payees::{self, Payee},
    period::Period,
    transactions::query::Total,
    transactions::{self, Split, Transaction},
};

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
    /// A leaf Expense [`Category::id`].
    pub category_id: u32,
    /// The Unit code (e.g. `"aud"`), always the Account's; locked after creation.
    pub unit: String,
    /// The [`Account::id`] it's paid from; one that takes Transactions.
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
    generate_from: NaiveDate,
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
        Resolution::Unresolved if Period::of(entry.due) > Period::of(today) => BillStatus::Upcoming,
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

/// The Schedule tab period that shows an entry: the current month for an Overdue entry carried
/// into it, the entry's own month otherwise.
pub fn schedule_period(entry: &BillScheduleEntry, today: NaiveDate) -> Period {
    let current = Period::of(today);
    if status(entry, today) == BillStatus::Overdue && Period::of(entry.due) < current {
        current
    } else {
        Period::of(entry.due)
    }
}

pub fn get(plans: &[BillPlan], id: u32) -> Option<&BillPlan> {
    plans.iter().find(|plan| plan.id == id)
}

pub fn entry(entries: &[BillScheduleEntry], id: EntryId) -> Option<&BillScheduleEntry> {
    entries.iter().find(|entry| entry.id() == id)
}

fn entry_mut(entries: &mut [BillScheduleEntry], id: EntryId) -> Option<&mut BillScheduleEntry> {
    entries.iter_mut().find(|entry| entry.id() == id)
}

/// The last date generation reaches: the end of next calendar month.
pub fn horizon(today: NaiveDate) -> NaiveDate {
    Period::of(today).next().last_day()
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
fn supersede_from(entries: &mut [BillScheduleEntry], plan_id: u32, today: NaiveDate) {
    for entry in entries
        .iter_mut()
        .filter(|e| e.plan_id == plan_id && e.is_unresolved() && e.due >= today)
    {
        entry.superseded = true;
    }
}

/// Errors for Bill Plan and Bill Schedule operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BillError {
    #[error("a bill plan needs a name")]
    NameRequired,
    #[error("a bill plan's category must be an expense category with no children")]
    CategoryNotExpenseLeaf,
    #[error("a bill plan needs an account that takes transactions")]
    AccountRequired,
    #[error("the unit must be the account's unit")]
    UnitMismatch,
    /// Carries the locked Unit.
    #[error("the unit is locked at {0}")]
    UnitLocked(String),
    #[error("the planned amount must be greater than zero")]
    AmountNotPositive,
    #[error("ends on can't be before first due")]
    EndsBeforeFirstDue,
    #[error("a one-shot bill plan has no ends on date")]
    EndsOnForOneShot,
    #[error("bill plan not found")]
    PlanNotFound,
    #[error("bill schedule entry not found")]
    EntryNotFound,
    /// Paid, Skipped or superseded already, or a preview with no row behind it.
    #[error("this bill schedule entry isn't open")]
    EntryNotOpen,
    #[error("that split can't be matched to this bill")]
    NotACandidate,
    #[error("this bill schedule entry isn't paid")]
    NotPaid,
    #[error("this bill schedule entry isn't skipped")]
    NotSkipped,
}

/// What the Add and Edit dialogs submit.
#[derive(Debug, Clone, PartialEq)]
pub struct BillPlanDraft {
    pub name: String,
    pub category_id: u32,
    pub unit: String,
    pub account_id: u32,
    pub payee_id: Option<u32>,
    pub planned_amount: Money,
    pub amount_kind: AmountKind,
    pub recurrence: Recurrence,
    pub first_due: NaiveDate,
    pub ends_on: Option<NaiveDate>,
    pub attention_lead: Option<u32>,
}

/// Checks everything a draft must satisfy; returns the trimmed name.
fn validate(
    draft: &BillPlanDraft,
    categories: &[Category],
    accounts: &[Account],
) -> Result<String, BillError> {
    let name = draft.name.trim();
    if name.is_empty() {
        return Err(BillError::NameRequired);
    }
    let is_expense_leaf = categories.iter().any(|c| {
        c.id == draft.category_id
            && c.category_type == CategoryTypes::Expense
            && categories::is_leaf(categories, c.id)
    });
    if !is_expense_leaf {
        return Err(BillError::CategoryNotExpenseLeaf);
    }
    let account = accounts
        .iter()
        .find(|a| a.id == draft.account_id && transactions::takes_transactions(&a.account_type))
        .ok_or(BillError::AccountRequired)?;
    if account.unit != draft.unit {
        return Err(BillError::UnitMismatch);
    }
    if !draft.planned_amount.0.is_positive() {
        return Err(BillError::AmountNotPositive);
    }
    match draft.ends_on {
        Some(_) if draft.recurrence == Recurrence::OneShot => Err(BillError::EndsOnForOneShot),
        Some(ends_on) if ends_on < draft.first_due => Err(BillError::EndsBeforeFirstDue),
        _ => Ok(name.to_string()),
    }
}

/// Adds an active Bill Plan and generates its entries from its First Due.
pub fn insert_plan(
    plans: &mut Vec<BillPlan>,
    entries: &mut Vec<BillScheduleEntry>,
    draft: &BillPlanDraft,
    categories: &[Category],
    accounts: &[Account],
    today: NaiveDate,
) -> Result<u32, BillError> {
    let name = validate(draft, categories, accounts)?;
    let id = plans.iter().map(|p| p.id).max().unwrap_or(0) + 1;
    let plan = BillPlan {
        id,
        name,
        category_id: draft.category_id,
        unit: draft.unit.clone(),
        account_id: draft.account_id,
        payee_id: draft.payee_id,
        planned_amount: draft.planned_amount.clone(),
        amount_kind: draft.amount_kind,
        recurrence: draft.recurrence,
        first_due: draft.first_due,
        ends_on: draft.ends_on,
        attention_lead: draft.attention_lead,
        is_active: true,
        generate_from: draft.first_due,
    };
    populate(std::slice::from_ref(&plan), entries, today);
    plans.push(plan);
    Ok(id)
}

/// Saves an edit. The Unit is locked. Changing Recurrence, First Due or Ends On supersedes the
/// unresolved entries due today or later and regenerates from today; any other change reads
/// through live, since unresolved entries hold no copy of the Plan's figures.
pub fn edit_plan(
    plans: &mut [BillPlan],
    entries: &mut Vec<BillScheduleEntry>,
    id: u32,
    draft: &BillPlanDraft,
    categories: &[Category],
    accounts: &[Account],
    today: NaiveDate,
) -> Result<(), BillError> {
    let plan = plans
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or(BillError::PlanNotFound)?;
    if draft.unit != plan.unit {
        return Err(BillError::UnitLocked(plan.unit.clone()));
    }
    let name = validate(draft, categories, accounts)?;
    let reschedules = (draft.recurrence, draft.first_due, draft.ends_on)
        != (plan.recurrence, plan.first_due, plan.ends_on);
    plan.name = name;
    plan.category_id = draft.category_id;
    plan.account_id = draft.account_id;
    plan.payee_id = draft.payee_id;
    plan.planned_amount = draft.planned_amount.clone();
    plan.amount_kind = draft.amount_kind;
    plan.recurrence = draft.recurrence;
    plan.first_due = draft.first_due;
    plan.ends_on = draft.ends_on;
    plan.attention_lead = draft.attention_lead;
    if reschedules {
        plan.generate_from = today;
        supersede_from(entries, id, today);
        populate(std::slice::from_ref(plan), entries, today);
    }
    Ok(())
}

/// Deactivating supersedes the unresolved entries due today or later (Overdue ones stay, still
/// needing resolution); reactivating generates from today without backfilling.
pub fn set_active(
    plans: &mut [BillPlan],
    entries: &mut Vec<BillScheduleEntry>,
    id: u32,
    is_active: bool,
    today: NaiveDate,
) -> Result<(), BillError> {
    let plan = plans
        .iter_mut()
        .find(|p| p.id == id)
        .ok_or(BillError::PlanNotFound)?;
    if plan.is_active == is_active {
        return Ok(());
    }
    plan.is_active = is_active;
    if is_active {
        plan.generate_from = today;
        populate(std::slice::from_ref(plan), entries, today);
    } else {
        supersede_from(entries, id, today);
    }
    Ok(())
}

/// The open entry `id` and its Plan.
fn open_entry<'a>(
    plans: &'a [BillPlan],
    entries: &[BillScheduleEntry],
    id: EntryId,
) -> Result<&'a BillPlan, BillError> {
    let found = entry(entries, id).ok_or(BillError::EntryNotFound)?;
    if !found.is_open() {
        return Err(BillError::EntryNotOpen);
    }
    get(plans, id.plan_id).ok_or(BillError::PlanNotFound)
}

/// Pay it directly: creates an Open Transaction on `date` in the Plan's Account, with one Split of
/// `-amount` carrying the Plan's Category and Payee, and Matches the entry to it. Returns the new
/// Transaction's id. (ADR-0019 says Pending; the Transactions stub's least-confirmed status is
/// Open.)
pub fn pay(
    plans: &[BillPlan],
    entries: &mut [BillScheduleEntry],
    transactions: &mut Vec<Transaction>,
    id: EntryId,
    amount: &Money,
    date: NaiveDate,
) -> Result<u32, BillError> {
    let plan = open_entry(plans, entries, id)?;
    if !amount.0.is_positive() {
        return Err(BillError::AmountNotPositive);
    }
    let transaction_id = transactions.iter().map(|t| t.id).max().unwrap_or(0) + 1;
    let transaction = Transaction {
        id: transaction_id,
        date,
        account_id: plan.account_id,
        status: TransactionStatus::Open,
        is_flagged: false,
        description: Some(plan.name.clone()),
        splits: vec![Split {
            amount: Money(-amount.0.clone()),
            category_id: plan.category_id,
            payee_id: plan.payee_id,
            tag_ids: Vec::new(),
        }],
    };
    // Newest first, as the Transactions stub keeps them.
    let at = transactions
        .iter()
        .position(|t| t.date <= date)
        .unwrap_or(transactions.len());
    transactions.insert(at, transaction);
    if let Some(found) = entry_mut(entries, id) {
        found.resolution = Resolution::Paid(SplitRef {
            transaction_id,
            split_index: 0,
        });
    }
    Ok(transaction_id)
}

/// Whether any entry is Matched to `split`.
pub fn is_matched(entries: &[BillScheduleEntry], split: SplitRef) -> bool {
    entries
        .iter()
        .any(|e| e.resolution == Resolution::Paid(split))
}

/// Whether a Transaction carries a Bill: any of its Splits is Matched.
pub fn carries_bill(entries: &[BillScheduleEntry], transaction_id: u32) -> bool {
    entries.iter().any(
        |e| matches!(e.resolution, Resolution::Paid(split) if split.transaction_id == transaction_id),
    )
}

/// How far either side of the due date a Match candidate's Transaction may be dated.
pub const MATCH_WINDOW_DAYS: i64 = 14;

/// The Splits entry `id` may be Matched to: unmatched Expense Splits in the Plan's Unit whose
/// Transaction is dated within [`MATCH_WINDOW_DAYS`] of the due date, from any Account or Payee.
/// Ordered by Payee match, then Account match, then closeness of amount, then of date.
pub fn match_candidates(
    plans: &[BillPlan],
    entries: &[BillScheduleEntry],
    transactions: &[Transaction],
    accounts: &[Account],
    categories: &[Category],
    id: EntryId,
) -> Vec<SplitRef> {
    let Ok(plan) = open_entry(plans, entries, id) else {
        return Vec::new();
    };
    let unit_of = |account_id: u32| {
        accounts
            .iter()
            .find(|a| a.id == account_id)
            .map(|a| a.unit.as_str())
    };
    let is_expense = |category_id: u32| {
        categories
            .iter()
            .any(|c| c.id == category_id && c.category_type == CategoryTypes::Expense)
    };
    let mut found: Vec<(bool, bool, BigDecimal, i64, SplitRef)> = Vec::new();
    for transaction in transactions {
        let days = (transaction.date - id.due).num_days().abs();
        if days > MATCH_WINDOW_DAYS || unit_of(transaction.account_id) != Some(plan.unit.as_str()) {
            continue;
        }
        for (split_index, split) in transaction.splits.iter().enumerate() {
            let split_ref = SplitRef {
                transaction_id: transaction.id,
                split_index,
            };
            if !is_expense(split.category_id) || is_matched(entries, split_ref) {
                continue;
            }
            let gap = (split.amount.0.abs() - &plan.planned_amount.0).abs();
            found.push((
                split.payee_id.is_some() && split.payee_id == plan.payee_id,
                transaction.account_id == plan.account_id,
                gap,
                days,
                split_ref,
            ));
        }
    }
    found.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then(b.1.cmp(&a.1))
            .then(a.2.cmp(&b.2))
            .then(a.3.cmp(&b.3))
    });
    found.into_iter().map(|(.., split_ref)| split_ref).collect()
}

/// The candidate the Match list pre-selects: the first, only when its Payee is the Plan's.
pub fn preselected_candidate(
    plan: &BillPlan,
    candidates: &[SplitRef],
    transactions: &[Transaction],
) -> Option<SplitRef> {
    let first = *candidates.first()?;
    let split = split_of(transactions, first)?;
    (plan.payee_id.is_some() && split.payee_id == plan.payee_id).then_some(first)
}

pub fn split_of(transactions: &[Transaction], split: SplitRef) -> Option<&Split> {
    transactions
        .iter()
        .find(|t| t.id == split.transaction_id)?
        .splits
        .get(split.split_index)
}

/// Match existing transaction: ties entry `id` to one of its [`match_candidates`].
pub fn match_split(
    plans: &[BillPlan],
    entries: &mut [BillScheduleEntry],
    transactions: &[Transaction],
    accounts: &[Account],
    categories: &[Category],
    id: EntryId,
    split: SplitRef,
) -> Result<(), BillError> {
    open_entry(plans, entries, id)?;
    if !match_candidates(plans, entries, transactions, accounts, categories, id).contains(&split) {
        return Err(BillError::NotACandidate);
    }
    if let Some(found) = entry_mut(entries, id) {
        found.resolution = Resolution::Paid(split);
    }
    Ok(())
}

/// Skips an open entry (for a One-shot Plan, cancels the bill), snapshotting the Planned Amount.
pub fn skip(
    plans: &[BillPlan],
    entries: &mut [BillScheduleEntry],
    id: EntryId,
) -> Result<(), BillError> {
    let planned = open_entry(plans, entries, id)?.planned_amount.clone();
    if let Some(found) = entry_mut(entries, id) {
        found.resolution = Resolution::Skipped { planned };
    }
    Ok(())
}

/// Undoes Paid: clears the Match and leaves the Transaction in place.
pub fn unmatch(entries: &mut [BillScheduleEntry], id: EntryId) -> Result<(), BillError> {
    let found = entry_mut(entries, id).ok_or(BillError::EntryNotFound)?;
    if !matches!(found.resolution, Resolution::Paid(_)) {
        return Err(BillError::NotPaid);
    }
    found.resolution = Resolution::Unresolved;
    Ok(())
}

/// Undoes Skipped.
pub fn unskip(entries: &mut [BillScheduleEntry], id: EntryId) -> Result<(), BillError> {
    let found = entry_mut(entries, id).ok_or(BillError::EntryNotFound)?;
    if !matches!(found.resolution, Resolution::Skipped { .. }) {
        return Err(BillError::NotSkipped);
    }
    found.resolution = Resolution::Unresolved;
    Ok(())
}

/// Unmatches every entry Matched to a Split of `transaction_id`: what deleting that Transaction
/// must do, so its entries return to Needs Attention.
pub fn unmatch_transaction(entries: &mut [BillScheduleEntry], transaction_id: u32) {
    for found in entries.iter_mut() {
        if matches!(found.resolution, Resolution::Paid(split) if split.transaction_id == transaction_id)
        {
            found.resolution = Resolution::Unresolved;
        }
    }
}

/// An entry's amount as a positive magnitude: the Matched Split's for Paid, the snapshot for
/// Skipped, the Plan's live Planned Amount otherwise.
pub fn amount(
    entry: &BillScheduleEntry,
    plans: &[BillPlan],
    transactions: &[Transaction],
) -> Option<Money> {
    match &entry.resolution {
        Resolution::Paid(split) => split_of(transactions, *split).map(|s| Money(s.amount.0.abs())),
        Resolution::Skipped { planned } => Some(planned.clone()),
        Resolution::Unresolved => get(plans, entry.plan_id).map(|p| p.planned_amount.clone()),
    }
}

/// The date an entry was paid: its Matched Split's Transaction date.
pub fn paid_on(entry: &BillScheduleEntry, transactions: &[Transaction]) -> Option<NaiveDate> {
    let Resolution::Paid(split) = entry.resolution else {
        return None;
    };
    transactions
        .iter()
        .find(|t| t.id == split.transaction_id)
        .map(|t| t.date)
}

/// One row of the Schedule tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduleRow {
    pub id: EntryId,
    pub status: BillStatus,
    /// An Overdue or Due entry from outside the viewed month, carried into it so an actionable
    /// row is never out of sight.
    pub carried: bool,
    /// Computed from the Recurrence past the generation horizon: no row exists yet, so it can't be
    /// paid or skipped.
    pub preview: bool,
    pub needs_attention: bool,
}

impl ScheduleRow {
    /// Whether Pay and Skip act on it: Due or Overdue, and a real entry rather than a preview.
    pub fn is_actionable(&self) -> bool {
        !self.preview && matches!(self.status, BillStatus::Due | BillStatus::Overdue)
    }

    /// Paid or Skipped: nothing left to do.
    pub fn is_resolved(&self) -> bool {
        matches!(self.status, BillStatus::Paid | BillStatus::Skipped)
    }
}

/// The Schedule tab's rows for `period` (`None` for All: every entry ever generated), before its
/// filters (#381). A month shows its own entries, every Overdue and Due entry from other months
/// carried in, and computed previews for active Plans past the horizon.
///
/// Unresolved rows come first, next due first; resolved rows follow, most recent first, so the
/// oldest settled row sits at the bottom.
pub fn schedule_rows(
    plans: &[BillPlan],
    entries: &[BillScheduleEntry],
    period: Option<Period>,
    today: NaiveDate,
) -> Vec<ScheduleRow> {
    let attention =
        |e: &BillScheduleEntry| get(plans, e.plan_id).is_some_and(|p| needs_attention(e, p, today));
    let mut rows: Vec<ScheduleRow> = entries
        .iter()
        .filter(|e| !e.superseded)
        .filter_map(|e| {
            let status = status(e, today);
            let own = period.is_none_or(|period| period.contains(e.due));
            let carried = !own && matches!(status, BillStatus::Overdue | BillStatus::Due);
            (own || carried).then(|| ScheduleRow {
                id: e.id(),
                status,
                carried,
                preview: false,
                needs_attention: attention(e),
            })
        })
        .collect();
    if let Some(period) = period
        && period.first_day() > horizon(today)
    {
        for plan in plans.iter().filter(|p| p.is_active) {
            let ends_on = plan.ends_on.unwrap_or(NaiveDate::MAX);
            let mut n = 0;
            while let Some(due) = plan.recurrence.occurrence(plan.first_due, n) {
                n += 1;
                if due > period.last_day() || due > ends_on {
                    break;
                }
                let id = EntryId {
                    plan_id: plan.id,
                    due,
                };
                if period.contains(due) && entry(entries, id).is_none() {
                    rows.push(ScheduleRow {
                        id,
                        status: BillStatus::Upcoming,
                        carried: false,
                        preview: true,
                        needs_attention: false,
                    });
                }
            }
        }
    }
    rows.sort_by(|a, b| {
        a.is_resolved().cmp(&b.is_resolved()).then_with(|| {
            if a.is_resolved() {
                b.id.due.cmp(&a.id.due)
            } else {
                a.id.due.cmp(&b.id.due)
            }
            .then_with(|| a.id.plan_id.cmp(&b.id.plan_id))
        })
    });
    rows
}

/// The Schedule tab's header meta for one period.
#[derive(Debug, Clone, PartialEq)]
pub struct PeriodSummary {
    pub due: usize,
    /// Carried entries included.
    pub overdue: usize,
    pub paid: usize,
    /// The period's own rows (carried ones excluded): Paid at their Matched amount, Skipped at
    /// their snapshot, the rest at their Plan's Planned Amount.
    pub planned: Total,
}

pub fn period_summary(
    rows: &[ScheduleRow],
    plans: &[BillPlan],
    entries: &[BillScheduleEntry],
    transactions: &[Transaction],
) -> PeriodSummary {
    let count = |status| rows.iter().filter(|r| r.status == status).count();
    let mut planned = Total::Empty;
    for row in rows.iter().filter(|r| !r.carried) {
        let Some(plan) = get(plans, row.id.plan_id) else {
            continue;
        };
        let figure = match entry(entries, row.id) {
            Some(found) => amount(found, plans, transactions),
            None => Some(plan.planned_amount.clone()),
        };
        let Some(figure) = figure else {
            continue;
        };
        planned = match planned {
            Total::Empty => Total::Single {
                unit: plan.unit.clone(),
                amount: figure,
            },
            Total::Single { unit, amount } if unit == plan.unit => Total::Single {
                unit,
                amount: Money(amount.0 + figure.0),
            },
            _ => Total::Mixed,
        };
    }
    PeriodSummary {
        due: count(BillStatus::Due),
        overdue: count(BillStatus::Overdue),
        paid: count(BillStatus::Paid),
        planned,
    }
}

/// The Planner tab's rows: active Plans by name, then inactive ones.
pub fn planner_order(plans: &[BillPlan]) -> Vec<&BillPlan> {
    let mut sorted: Vec<&BillPlan> = plans.iter().collect();
    sorted.sort_by(|a, b| {
        b.is_active
            .cmp(&a.is_active)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    sorted
}

pub fn inactive_count(plans: &[BillPlan]) -> usize {
    plans.iter().filter(|p| !p.is_active).count()
}

/// Which tab of the Bills surface shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BillsTab {
    #[default]
    Schedule,
    Planner,
}

impl BillsTab {
    /// `tab` cycles forward, wrapping.
    pub fn next(self) -> Self {
        match self {
            BillsTab::Schedule => BillsTab::Planner,
            BillsTab::Planner => BillsTab::Schedule,
        }
    }
}

/// The open Bills dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BillsDialog {
    Add(BillPlanForm),
    /// Editing the Bill Plan with this [`BillPlan::id`].
    Edit(u32, BillPlanForm),
    Pay(PayForm),
    /// Confirming a skip of this entry: nothing to edit, so no form.
    Skip(EntryId),
}

impl BillsDialog {
    /// The form behind the Add and Edit bill plan dialogs.
    pub fn plan_form_mut(&mut self) -> Option<&mut BillPlanForm> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => Some(form),
            Self::Pay(_) | Self::Skip(_) => None,
        }
    }

    /// The form behind the dialog, which Skip has none of: it confirms with nothing to fill in.
    fn inner(&self) -> Option<&dyn Dialog> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => Some(form),
            Self::Pay(form) => Some(form),
            Self::Skip(_) => None,
        }
    }

    fn inner_mut(&mut self) -> Option<&mut dyn Dialog> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => Some(form),
            Self::Pay(form) => Some(form),
            Self::Skip(_) => None,
        }
    }
}

impl Dialog for BillsDialog {
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        self.inner_mut()?.handle_own_key(key)
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        self.inner_mut()?.focused_text()
    }

    fn cycle_field(&mut self) {
        if let Some(form) = self.inner_mut() {
            form.cycle_field();
        }
    }

    fn is_valid(&self) -> bool {
        self.inner().is_none_or(Dialog::is_valid)
    }

    fn close_open_select(&mut self) -> bool {
        self.inner_mut().is_some_and(Dialog::close_open_select)
    }
}

// ---------------------------------------------------------------------------------------------
// The stub dataset
// ---------------------------------------------------------------------------------------------

/// The seeded Bills: Plans, their Schedule, and the Transactions settling it.
#[derive(Debug, Clone, PartialEq)]
pub struct BillsSeed {
    pub plans: Vec<BillPlan>,
    pub entries: Vec<BillScheduleEntry>,
}

/// Builds an exact `Money` from a count of cents.
fn cents_money(cents: i64) -> Money {
    Money(BigDecimal::new(cents.into(), 2))
}

/// `day` of the month `offset` months from `today`'s, clamped to that month's last day.
fn day_in(today: NaiveDate, offset: i32, day: u32) -> NaiveDate {
    let period = Period::of(today).shift(offset);
    let last = period.last_day().day();
    NaiveDate::from_ymd_opt(period.year, period.month, day.min(last)).unwrap_or(today)
}

/// How this month's entry of a seeded Plan is left.
#[derive(Clone, Copy, PartialEq)]
enum ThisMonth {
    Open,
    Paid,
    Skipped,
}

struct PlanSeed {
    name: &'static str,
    category: &'static str,
    account: &'static str,
    payee: Option<&'static str>,
    cents: i64,
    kind: AmountKind,
    recurrence: Recurrence,
    /// `(months from today's month, day)`.
    first_due: (i32, u32),
    ends_on: Option<(i32, u32)>,
    lead: Option<u32>,
    active: bool,
    this_month: ThisMonth,
    /// Earlier-month cycles (by occurrence index) that were skipped rather than paid.
    skipped: &'static [u32],
}

/// The handoff's ten sample Bill Plans (8b), laid out so that on 19 September the Schedule (8a)
/// shows its nine rows and statuses: Streaming Bundle Paid, Car Wash Skipped, Gym and Netflix
/// Overdue, Telstra Due inside its 3-day lead, Origin and AAMI Due, Rent and Council Rates
/// Upcoming. Rent's first due date is next month, so Rent has no September row.
const PLAN_SEEDS: &[PlanSeed] = &[
    PlanSeed {
        name: "Rent",
        category: "Rent",
        account: "ANZ Everyday",
        payee: Some("Ray White Rentals"),
        cents: 240_000,
        kind: AmountKind::Fixed,
        recurrence: Recurrence::Monthly,
        first_due: (1, 1),
        ends_on: None,
        lead: None,
        active: true,
        this_month: ThisMonth::Open,
        skipped: &[],
    },
    PlanSeed {
        name: "Netflix",
        category: "Household",
        account: "Amex Platinum",
        payee: Some("Netflix"),
        cents: 2_299,
        kind: AmountKind::Fixed,
        recurrence: Recurrence::Monthly,
        first_due: (-14, 18),
        ends_on: None,
        lead: None,
        active: true,
        this_month: ThisMonth::Open,
        skipped: &[],
    },
    PlanSeed {
        name: "Gym \u{2014} Fitness First",
        category: "Household",
        account: "ANZ Everyday",
        payee: Some("Fitness First"),
        cents: 6_400,
        kind: AmountKind::Fixed,
        recurrence: Recurrence::Monthly,
        first_due: (-10, 15),
        ends_on: None,
        lead: None,
        active: true,
        this_month: ThisMonth::Open,
        skipped: &[4],
    },
    PlanSeed {
        name: "Telstra Internet",
        category: "Household",
        account: "ANZ Everyday",
        payee: Some("Telstra"),
        cents: 8_900,
        kind: AmountKind::Fixed,
        recurrence: Recurrence::Monthly,
        first_due: (-15, 22),
        ends_on: None,
        lead: Some(3),
        active: true,
        this_month: ThisMonth::Open,
        skipped: &[],
    },
    PlanSeed {
        name: "Origin Energy",
        category: "Electricity",
        account: "ANZ Everyday",
        payee: Some("Origin Energy"),
        cents: 15_000,
        kind: AmountKind::Estimated,
        recurrence: Recurrence::Quarterly,
        first_due: (-15, 24),
        ends_on: None,
        lead: None,
        active: true,
        this_month: ThisMonth::Open,
        skipped: &[],
    },
    PlanSeed {
        name: "Car Insurance \u{2014} AAMI",
        category: "Transport",
        account: "Amex Platinum",
        payee: Some("AAMI"),
        cents: 118_000,
        kind: AmountKind::Fixed,
        recurrence: Recurrence::Annually,
        first_due: (-12, 30),
        ends_on: None,
        lead: Some(5),
        active: true,
        this_month: ThisMonth::Open,
        skipped: &[],
    },
    PlanSeed {
        name: "Council Rates",
        category: "Household",
        account: "ANZ Everyday",
        payee: Some("Brisbane City Council"),
        cents: 52_000,
        kind: AmountKind::Estimated,
        recurrence: Recurrence::Quarterly,
        first_due: (2, 5),
        ends_on: None,
        lead: None,
        active: true,
        this_month: ThisMonth::Open,
        skipped: &[],
    },
    PlanSeed {
        name: "Streaming Bundle",
        category: "Household",
        account: "Amex Platinum",
        payee: None,
        cents: 2_999,
        kind: AmountKind::Fixed,
        recurrence: Recurrence::Monthly,
        first_due: (-6, 5),
        ends_on: None,
        lead: None,
        active: true,
        this_month: ThisMonth::Paid,
        skipped: &[],
    },
    PlanSeed {
        name: "Car Wash Membership",
        category: "Transport",
        account: "Amex Platinum",
        payee: None,
        cents: 2_500,
        kind: AmountKind::Fixed,
        recurrence: Recurrence::Monthly,
        first_due: (-4, 10),
        ends_on: None,
        lead: None,
        active: true,
        this_month: ThisMonth::Skipped,
        skipped: &[1],
    },
    PlanSeed {
        name: "Basic Fitness (closed)",
        category: "Household",
        account: "ANZ Everyday",
        payee: None,
        cents: 1_995,
        kind: AmountKind::Fixed,
        recurrence: Recurrence::Monthly,
        first_due: (-16, 12),
        ends_on: Some((-4, 12)),
        lead: None,
        active: false,
        this_month: ThisMonth::Open,
        skipped: &[],
    },
];

/// A deterministic spread for a paid cycle's actual amount: an Estimated Plan varies from 85% to
/// 114% of its estimate, and Telstra's older cycles were billed at 76.20 then 79.99, so the History
/// figures have something to show.
fn paid_cents(seed: &PlanSeed, index: u32, cycles: u32) -> i64 {
    match seed.kind {
        AmountKind::Estimated => seed.cents * (85 + i64::from(index * 7 % 30)) / 100,
        AmountKind::Fixed if seed.name == "Telstra Internet" && index + 3 < cycles => {
            if index < 6 {
                7_620
            } else {
                7_999
            }
        }
        AmountKind::Fixed => seed.cents,
    }
}

/// Seeds the handoff's Bill Plans against the shared stubs and settles their history: every
/// earlier-month cycle is Paid (a Reconciled Transaction written to `transactions`, dated a day or
/// two either side of the due date) unless its Plan skipped it, and this month's is left as the
/// handoff shows it. Also adds one unmatched Telstra payment yesterday, the 8d Match candidate.
/// A Plan whose Account, Category or Payee the stubs lack is left out.
pub fn default_bills(
    accounts: &[Account],
    categories: &[Category],
    payees: &[Payee],
    transactions: &mut Vec<Transaction>,
    today: NaiveDate,
) -> BillsSeed {
    let mut plans: Vec<BillPlan> = Vec::new();
    let mut entries: Vec<BillScheduleEntry> = Vec::new();
    let this_month = Period::of(today);

    for seed in PLAN_SEEDS {
        let Some(account) = accounts.iter().find(|a| a.name == seed.account) else {
            continue;
        };
        let Some(category_id) = categories::find_by_name(categories, seed.category) else {
            continue;
        };
        let payee_id = match seed.payee {
            Some(name) => match payees::find_by_name(payees, name) {
                Some(id) => Some(id),
                None => continue,
            },
            None => None,
        };
        let first_due = day_in(today, seed.first_due.0, seed.first_due.1);
        let draft = BillPlanDraft {
            name: seed.name.to_string(),
            category_id,
            unit: account.unit.clone(),
            account_id: account.id,
            payee_id,
            planned_amount: cents_money(seed.cents),
            amount_kind: seed.kind,
            recurrence: seed.recurrence,
            first_due,
            ends_on: seed.ends_on.map(|(offset, day)| day_in(today, offset, day)),
            attention_lead: seed.lead,
        };
        let Ok(id) = insert_plan(
            &mut plans,
            &mut entries,
            &draft,
            categories,
            accounts,
            today,
        ) else {
            continue;
        };

        let mine: Vec<EntryId> = entries
            .iter()
            .filter(|e| e.plan_id == id && Period::of(e.due) <= this_month)
            .map(BillScheduleEntry::id)
            .collect();
        let cycles = mine.len() as u32;
        for (index, entry_id) in (0u32..).zip(mine) {
            let resolve = if Period::of(entry_id.due) < this_month {
                if seed.skipped.contains(&index) {
                    ThisMonth::Skipped
                } else {
                    ThisMonth::Paid
                }
            } else {
                seed.this_month
            };
            match resolve {
                ThisMonth::Open => {}
                ThisMonth::Skipped => {
                    let _ = skip(&plans, &mut entries, entry_id);
                }
                ThisMonth::Paid => {
                    let shift = i64::from(index % 3) - 1;
                    let date = (entry_id.due + Duration::days(shift)).min(today);
                    let cents = paid_cents(seed, index, cycles);
                    if let Ok(transaction_id) = pay(
                        &plans,
                        &mut entries,
                        transactions,
                        entry_id,
                        &cents_money(cents),
                        date,
                    ) && let Some(t) = transactions.iter_mut().find(|t| t.id == transaction_id)
                    {
                        t.status = if Period::of(date) < this_month {
                            TransactionStatus::Reconciled
                        } else {
                            TransactionStatus::Cleared
                        };
                    }
                }
            }
        }
        if !seed.active {
            let _ = set_active(&mut plans, &mut entries, id, false, today);
        }
    }

    if let (Some(account), Some(category_id), Some(payee_id)) = (
        accounts.iter().find(|a| a.name == "ANZ Everyday"),
        categories::find_by_name(categories, "Household"),
        payees::find_by_name(payees, "Telstra"),
    ) {
        let date = today - Duration::days(1);
        let id = transactions.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        let at = transactions
            .iter()
            .position(|t| t.date <= date)
            .unwrap_or(transactions.len());
        transactions.insert(
            at,
            Transaction {
                id,
                date,
                account_id: account.id,
                status: TransactionStatus::Cleared,
                is_flagged: false,
                description: Some("TELSTRA DIRECT DEBIT".to_string()),
                splits: vec![Split {
                    amount: cents_money(-7_999),
                    category_id,
                    payee_id: Some(payee_id),
                    tag_ids: Vec::new(),
                }],
            },
        );
    }

    entries.sort_by_key(|entry| (entry.due, entry.plan_id));
    BillsSeed { plans, entries }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        accounts::default_accounts, categories::default_categories, payees::default_payees,
        tags::default_tags, transactions::default_transactions,
    };

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap()
    }

    fn today() -> NaiveDate {
        date(2026, 9, 19)
    }

    struct World {
        accounts: Vec<Account>,
        categories: Vec<Category>,
        transactions: Vec<Transaction>,
        plans: Vec<BillPlan>,
        entries: Vec<BillScheduleEntry>,
    }

    fn world() -> World {
        let accounts = default_accounts();
        let categories = default_categories();
        let payees = default_payees();
        let mut transactions =
            default_transactions(&accounts, &categories, &payees, &default_tags(), today());
        let seed = default_bills(&accounts, &categories, &payees, &mut transactions, today());
        World {
            accounts,
            categories,
            transactions,
            plans: seed.plans,
            entries: seed.entries,
        }
    }

    fn empty() -> World {
        World {
            accounts: default_accounts(),
            categories: default_categories(),
            transactions: Vec::new(),
            plans: Vec::new(),
            entries: Vec::new(),
        }
    }

    fn draft(recurrence: Recurrence, first_due: NaiveDate) -> BillPlanDraft {
        BillPlanDraft {
            name: "Water".to_string(),
            category_id: 5,
            unit: "aud".to_string(),
            account_id: 2,
            payee_id: Some(15),
            planned_amount: cents_money(9_000),
            amount_kind: AmountKind::Estimated,
            recurrence,
            first_due,
            ends_on: None,
            attention_lead: None,
        }
    }

    fn add(w: &mut World, draft: &BillPlanDraft) -> u32 {
        insert_plan(
            &mut w.plans,
            &mut w.entries,
            draft,
            &w.categories,
            &w.accounts,
            today(),
        )
        .unwrap()
    }

    fn dues(w: &World, plan_id: u32) -> Vec<NaiveDate> {
        let mut dates: Vec<NaiveDate> = w
            .entries
            .iter()
            .filter(|e| e.plan_id == plan_id && !e.superseded)
            .map(|e| e.due)
            .collect();
        dates.sort();
        dates
    }

    fn plan_id(w: &World, name: &str) -> u32 {
        w.plans.iter().find(|p| p.name == name).unwrap().id
    }

    fn id(plan_id: u32, due: NaiveDate) -> EntryId {
        EntryId { plan_id, due }
    }

    // --- recurrence ---

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
    fn periods_step_across_years() {
        let december = Period {
            year: 2026,
            month: 12,
        };
        assert_eq!(
            december.next(),
            Period {
                year: 2027,
                month: 1
            }
        );
        assert_eq!(december.next().prev(), december);
        assert_eq!(december.last_day(), date(2026, 12, 31));
        assert_eq!(horizon(today()), date(2026, 10, 31));
    }

    // --- status and Needs Attention ---

    fn open(due: NaiveDate) -> BillScheduleEntry {
        BillScheduleEntry {
            plan_id: 1,
            due,
            resolution: Resolution::Unresolved,
            superseded: false,
        }
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

    #[test]
    fn needs_attention_is_a_date_rule_across_month_boundaries() {
        let w = empty();
        let mut plan_draft = draft(Recurrence::Monthly, today());
        plan_draft.attention_lead = Some(14);
        let mut w = w;
        let pid = add(&mut w, &plan_draft);
        let plan = get(&w.plans, pid).unwrap();
        let mut early_next_month = open(date(2026, 10, 2));
        early_next_month.plan_id = pid;
        assert!(needs_attention(&early_next_month, plan, today()));
        let mut later = open(date(2026, 10, 4));
        later.plan_id = pid;
        assert!(!needs_attention(&later, plan, today()));
    }

    #[test]
    fn no_lead_counts_from_the_due_day_and_resolved_rows_never_count() {
        let mut w = empty();
        let pid = add(&mut w, &draft(Recurrence::Monthly, today()));
        let plan = get(&w.plans, pid).unwrap().clone();
        let mut tomorrow = open(today() + Duration::days(1));
        tomorrow.plan_id = pid;
        assert!(!needs_attention(&tomorrow, &plan, today()));
        let mut due_today = open(today());
        due_today.plan_id = pid;
        assert!(needs_attention(&due_today, &plan, today()));
        due_today.resolution = Resolution::Skipped {
            planned: cents_money(1),
        };
        assert!(!needs_attention(&due_today, &plan, today()));
    }

    // --- generation ---

    #[test]
    fn generation_runs_to_the_end_of_next_month() {
        let mut w = empty();
        let pid = add(&mut w, &draft(Recurrence::Weekly, date(2026, 9, 14)));
        let d = dues(&w, pid);
        assert_eq!(d.first(), Some(&date(2026, 9, 14)));
        assert_eq!(d.last(), Some(&date(2026, 10, 26)));
        assert_eq!(d.len(), 7);
    }

    #[test]
    fn a_long_recurrence_keeps_one_future_entry_past_the_horizon() {
        let mut w = empty();
        let pid = add(&mut w, &draft(Recurrence::Annually, date(2026, 3, 1)));
        assert_eq!(dues(&w, pid), vec![date(2026, 3, 1), date(2027, 3, 1)]);
    }

    #[test]
    fn ends_on_is_inclusive_and_generation_stops_there() {
        let mut w = empty();
        let mut d = draft(Recurrence::Monthly, date(2026, 7, 10));
        d.ends_on = Some(date(2026, 9, 10));
        let pid = add(&mut w, &d);
        assert_eq!(
            dues(&w, pid),
            vec![date(2026, 7, 10), date(2026, 8, 10), date(2026, 9, 10)]
        );
    }

    #[test]
    fn populate_is_idempotent() {
        let mut w = world();
        let before = w.entries.clone();
        populate(&w.plans, &mut w.entries, today());
        assert_eq!(w.entries, before);
    }

    #[test]
    fn editing_the_recurrence_supersedes_future_open_entries_only() {
        let mut w = empty();
        let pid = add(&mut w, &draft(Recurrence::Monthly, date(2026, 8, 25)));
        // Aug 25 is Overdue and must survive the edit.
        let mut edit = draft(Recurrence::Monthly, date(2026, 8, 28));
        edit.name = "Water".into();
        edit_plan(
            &mut w.plans,
            &mut w.entries,
            pid,
            &edit,
            &w.categories,
            &w.accounts,
            today(),
        )
        .unwrap();
        assert_eq!(
            dues(&w, pid),
            vec![date(2026, 8, 25), date(2026, 9, 28), date(2026, 10, 28)]
        );
        let superseded: Vec<NaiveDate> = w
            .entries
            .iter()
            .filter(|e| e.superseded)
            .map(|e| e.due)
            .collect();
        assert_eq!(superseded, vec![date(2026, 9, 25), date(2026, 10, 25)]);
    }

    #[test]
    fn an_edit_back_revives_the_same_superseded_row() {
        let mut w = empty();
        let original = draft(Recurrence::Monthly, date(2026, 9, 25));
        let pid = add(&mut w, &original);
        let moved = draft(Recurrence::Monthly, date(2026, 9, 27));
        let (c, a) = (w.categories.clone(), w.accounts.clone());
        edit_plan(&mut w.plans, &mut w.entries, pid, &moved, &c, &a, today()).unwrap();
        edit_plan(
            &mut w.plans,
            &mut w.entries,
            pid,
            &original,
            &c,
            &a,
            today(),
        )
        .unwrap();
        assert_eq!(dues(&w, pid), vec![date(2026, 9, 25), date(2026, 10, 25)]);
        let rows = w
            .entries
            .iter()
            .filter(|e| e.due == date(2026, 9, 25))
            .count();
        assert_eq!(rows, 1);
    }

    #[test]
    fn a_settled_period_is_never_billed_twice() {
        let mut w = empty();
        let pid = add(&mut w, &draft(Recurrence::Monthly, date(2026, 9, 20)));
        skip(&w.plans, &mut w.entries, id(pid, date(2026, 9, 20))).unwrap();
        let (c, a) = (w.categories.clone(), w.accounts.clone());
        let moved = draft(Recurrence::Monthly, date(2026, 9, 26));
        edit_plan(&mut w.plans, &mut w.entries, pid, &moved, &c, &a, today()).unwrap();
        assert_eq!(dues(&w, pid), vec![date(2026, 9, 20), date(2026, 10, 26)]);
    }

    #[test]
    fn deactivating_keeps_overdue_and_reactivating_does_not_backfill() {
        let mut w = empty();
        let pid = add(&mut w, &draft(Recurrence::Weekly, date(2026, 9, 12)));
        set_active(&mut w.plans, &mut w.entries, pid, false, today()).unwrap();
        assert_eq!(dues(&w, pid), vec![date(2026, 9, 12)]);
        let later = date(2026, 10, 20);
        set_active(&mut w.plans, &mut w.entries, pid, true, later).unwrap();
        let d = dues(&w, pid);
        assert_eq!(d[0], date(2026, 9, 12));
        assert_eq!(d[1], date(2026, 10, 24));
    }

    #[test]
    fn a_plain_edit_reads_through_live() {
        let mut w = empty();
        let pid = add(&mut w, &draft(Recurrence::Monthly, date(2026, 9, 25)));
        let mut edit = draft(Recurrence::Monthly, date(2026, 9, 25));
        edit.planned_amount = cents_money(12_345);
        let (c, a) = (w.categories.clone(), w.accounts.clone());
        edit_plan(&mut w.plans, &mut w.entries, pid, &edit, &c, &a, today()).unwrap();
        assert!(w.entries.iter().all(|e| !e.superseded));
        let e = entry(&w.entries, id(pid, date(2026, 9, 25))).unwrap();
        assert_eq!(
            amount(e, &w.plans, &w.transactions),
            Some(cents_money(12_345))
        );
    }

    // --- validation ---

    fn refused(change: impl Fn(&mut BillPlanDraft)) -> BillError {
        let mut w = empty();
        let mut d = draft(Recurrence::Monthly, today());
        change(&mut d);
        insert_plan(
            &mut w.plans,
            &mut w.entries,
            &d,
            &w.categories,
            &w.accounts,
            today(),
        )
        .unwrap_err()
    }

    #[test]
    fn validation_refuses_bad_drafts() {
        assert_eq!(refused(|d| d.name = "  ".into()), BillError::NameRequired);
        // Salary is Income; Utilities has children.
        assert_eq!(
            refused(|d| d.category_id = 11),
            BillError::CategoryNotExpenseLeaf
        );
        assert_eq!(
            refused(|d| d.category_id = 3),
            BillError::CategoryNotExpenseLeaf
        );
        // Home Loan takes no Transactions.
        assert_eq!(refused(|d| d.account_id = 5), BillError::AccountRequired);
        assert_eq!(refused(|d| d.account_id = 99), BillError::AccountRequired);
        assert_eq!(refused(|d| d.unit = "usd".into()), BillError::UnitMismatch);
        assert_eq!(
            refused(|d| d.planned_amount = cents_money(0)),
            BillError::AmountNotPositive
        );
        assert_eq!(
            refused(|d| d.ends_on = Some(date(2026, 1, 1))),
            BillError::EndsBeforeFirstDue
        );
        assert_eq!(
            refused(|d| {
                d.recurrence = Recurrence::OneShot;
                d.ends_on = Some(date(2027, 1, 1));
            }),
            BillError::EndsOnForOneShot
        );
    }

    #[test]
    fn the_unit_is_locked_after_creation() {
        let mut w = empty();
        let pid = add(&mut w, &draft(Recurrence::Monthly, today()));
        let mut edit = draft(Recurrence::Monthly, today());
        edit.unit = "usd".into();
        let (c, a) = (w.categories.clone(), w.accounts.clone());
        assert_eq!(
            edit_plan(&mut w.plans, &mut w.entries, pid, &edit, &c, &a, today()),
            Err(BillError::UnitLocked("aud".into()))
        );
    }

    // --- settlement ---

    #[test]
    fn pay_writes_an_open_transaction_and_matches_its_split() {
        let mut w = world();
        let telstra = plan_id(&w, "Telstra Internet");
        let due = id(telstra, date(2026, 9, 22));
        let before = w.transactions.len();
        let tid = pay(
            &w.plans,
            &mut w.entries,
            &mut w.transactions,
            due,
            &cents_money(8_900),
            today(),
        )
        .unwrap();
        assert_eq!(w.transactions.len(), before + 1);
        let t = w.transactions.iter().find(|t| t.id == tid).unwrap();
        assert_eq!(t.status, TransactionStatus::Open);
        assert_eq!(t.total(), cents_money(-8_900));
        assert_eq!(t.account_id, 2);
        assert!(carries_bill(&w.entries, tid));
        let e = entry(&w.entries, due).unwrap();
        assert_eq!(status(e, today()), BillStatus::Paid);
        assert_eq!(paid_on(e, &w.transactions), Some(today()));
        // Still newest first.
        assert!(w.transactions.windows(2).all(|p| p[0].date >= p[1].date));
    }

    #[test]
    fn pay_refuses_a_non_positive_amount_and_a_resolved_entry() {
        let mut w = world();
        let telstra = plan_id(&w, "Telstra Internet");
        let due = id(telstra, date(2026, 9, 22));
        let zero = pay(
            &w.plans,
            &mut w.entries,
            &mut w.transactions,
            due,
            &cents_money(0),
            today(),
        );
        assert_eq!(zero, Err(BillError::AmountNotPositive));
        skip(&w.plans, &mut w.entries, due).unwrap();
        let again = pay(
            &w.plans,
            &mut w.entries,
            &mut w.transactions,
            due,
            &cents_money(1),
            today(),
        );
        assert_eq!(again, Err(BillError::EntryNotOpen));
    }

    #[test]
    fn match_candidates_put_the_payee_match_first_and_preselect_it() {
        let w = world();
        let telstra = plan_id(&w, "Telstra Internet");
        let due = id(telstra, date(2026, 9, 22));
        let candidates = match_candidates(
            &w.plans,
            &w.entries,
            &w.transactions,
            &w.accounts,
            &w.categories,
            due,
        );
        assert!(!candidates.is_empty());
        let plan = get(&w.plans, telstra).unwrap();
        let first = preselected_candidate(plan, &candidates, &w.transactions).unwrap();
        let split = split_of(&w.transactions, first).unwrap();
        assert_eq!(split.payee_id, plan.payee_id);
        assert_eq!(split.amount, cents_money(-7_999));
        // Nothing already Matched, nothing income, nothing outside the window.
        for c in &candidates {
            assert!(!is_matched(&w.entries, *c));
            let t = w
                .transactions
                .iter()
                .find(|t| t.id == c.transaction_id)
                .unwrap();
            assert!((t.date - due.due).num_days().abs() <= MATCH_WINDOW_DAYS);
            assert!(t.splits[c.split_index].amount.0.is_negative());
        }
    }

    #[test]
    fn match_split_pays_and_unmatch_reverses_leaving_the_transaction() {
        let mut w = world();
        let telstra = plan_id(&w, "Telstra Internet");
        let due = id(telstra, date(2026, 9, 22));
        let candidates = match_candidates(
            &w.plans,
            &w.entries,
            &w.transactions,
            &w.accounts,
            &w.categories,
            due,
        );
        let chosen = candidates[0];
        let before = w.transactions.len();
        match_split(
            &w.plans,
            &mut w.entries,
            &w.transactions,
            &w.accounts,
            &w.categories,
            due,
            chosen,
        )
        .unwrap();
        let e = entry(&w.entries, due).unwrap();
        assert_eq!(
            amount(e, &w.plans, &w.transactions),
            Some(cents_money(7_999))
        );
        // A Matched Split is no longer anyone's candidate.
        let netflix = plan_id(&w, "Netflix");
        let other = match_candidates(
            &w.plans,
            &w.entries,
            &w.transactions,
            &w.accounts,
            &w.categories,
            id(netflix, date(2026, 9, 18)),
        );
        assert!(!other.contains(&chosen));
        unmatch(&mut w.entries, due).unwrap();
        assert_eq!(
            status(entry(&w.entries, due).unwrap(), today()),
            BillStatus::Due
        );
        assert_eq!(w.transactions.len(), before);
    }

    #[test]
    fn match_split_refuses_a_non_candidate() {
        let mut w = world();
        let telstra = plan_id(&w, "Telstra Internet");
        let due = id(telstra, date(2026, 9, 22));
        let far = SplitRef {
            transaction_id: w.transactions.last().unwrap().id,
            split_index: 0,
        };
        let result = match_split(
            &w.plans,
            &mut w.entries,
            &w.transactions,
            &w.accounts,
            &w.categories,
            due,
            far,
        );
        assert_eq!(result, Err(BillError::NotACandidate));
    }

    #[test]
    fn skip_snapshots_the_planned_amount_and_unskip_reverses() {
        let mut w = world();
        let origin = plan_id(&w, "Origin Energy");
        let due = id(origin, date(2026, 9, 24));
        skip(&w.plans, &mut w.entries, due).unwrap();
        let (c, a) = (w.categories.clone(), w.accounts.clone());
        let plan = get(&w.plans, origin).unwrap().clone();
        let mut edit = draft(plan.recurrence, plan.first_due);
        edit.name = plan.name.clone();
        edit.category_id = plan.category_id;
        edit.payee_id = plan.payee_id;
        edit.planned_amount = cents_money(99_999);
        edit_plan(&mut w.plans, &mut w.entries, origin, &edit, &c, &a, today()).unwrap();
        let e = entry(&w.entries, due).unwrap();
        assert_eq!(
            amount(e, &w.plans, &w.transactions),
            Some(cents_money(15_000))
        );
        unskip(&mut w.entries, due).unwrap();
        assert_eq!(unskip(&mut w.entries, due), Err(BillError::NotSkipped));
    }

    #[test]
    fn a_one_shot_can_be_skipped_as_cancelled() {
        let mut w = empty();
        let pid = add(&mut w, &draft(Recurrence::OneShot, date(2026, 10, 3)));
        assert_eq!(dues(&w, pid), vec![date(2026, 10, 3)]);
        skip(&w.plans, &mut w.entries, id(pid, date(2026, 10, 3))).unwrap();
        populate(&w.plans, &mut w.entries, today());
        assert_eq!(dues(&w, pid), vec![date(2026, 10, 3)]);
    }

    #[test]
    fn deleting_a_matched_transaction_unmatches_its_entry() {
        let mut w = world();
        let streaming = plan_id(&w, "Streaming Bundle");
        let due = id(streaming, date(2026, 9, 5));
        let Resolution::Paid(split) = entry(&w.entries, due).unwrap().resolution else {
            panic!("the seed pays Streaming Bundle this month");
        };
        unmatch_transaction(&mut w.entries, split.transaction_id);
        assert_eq!(
            status(entry(&w.entries, due).unwrap(), today()),
            BillStatus::Overdue
        );
    }

    // --- the seed, against the handoff ---

    #[test]
    fn the_seed_has_the_handoff_plans_with_one_inactive() {
        let w = world();
        assert_eq!(w.plans.len(), 10);
        assert_eq!(inactive_count(&w.plans), 1);
        assert!(
            !get(&w.plans, plan_id(&w, "Basic Fitness (closed)"))
                .unwrap()
                .is_active
        );
        let order: Vec<&str> = planner_order(&w.plans)
            .iter()
            .map(|p| p.name.as_str())
            .collect();
        assert_eq!(order.last(), Some(&"Basic Fitness (closed)"));
    }

    #[test]
    fn the_september_schedule_matches_8a() {
        let w = world();
        let rows = schedule_rows(&w.plans, &w.entries, Some(Period::of(today())), today());
        let shown: Vec<(String, u32, BillStatus, bool)> = rows
            .iter()
            .map(|r| {
                let name = get(&w.plans, r.id.plan_id).unwrap().name.clone();
                (name, r.id.due.day(), r.status, r.needs_attention)
            })
            .collect();
        use BillStatus::*;
        // Unresolved first, next due first; then resolved, most recent first.
        let expected = [
            ("Gym \u{2014} Fitness First", 15, Overdue, true),
            ("Netflix", 18, Overdue, true),
            ("Telstra Internet", 22, Due, true),
            ("Origin Energy", 24, Due, false),
            ("Car Insurance \u{2014} AAMI", 30, Due, false),
            ("Car Wash Membership", 10, Skipped, false),
            ("Streaming Bundle", 5, Paid, false),
        ];
        let expected: Vec<(String, u32, BillStatus, bool)> = expected
            .iter()
            .map(|(n, d, s, a)| (n.to_string(), *d, *s, *a))
            .collect();
        assert_eq!(shown, expected);
        let attention: Vec<EntryId> = rows
            .iter()
            .filter(|r| r.needs_attention)
            .map(|r| r.id)
            .collect();
        assert_eq!(attention_entries(&w.plans, &w.entries, today()), attention);
    }

    #[test]
    fn rent_and_council_rates_are_upcoming_in_later_months() {
        let w = world();
        let october = schedule_rows(
            &w.plans,
            &w.entries,
            Some(Period::of(today()).next()),
            today(),
        );
        let rent = plan_id(&w, "Rent");
        assert!(
            october
                .iter()
                .any(|r| r.id == id(rent, date(2026, 10, 1)) && r.status == BillStatus::Upcoming)
        );
        let council = plan_id(&w, "Council Rates");
        assert!(entry(&w.entries, id(council, date(2026, 11, 5))).is_some());
    }

    #[test]
    fn an_earlier_overdue_row_carries_into_the_current_month() {
        let mut w = world();
        let netflix = plan_id(&w, "Netflix");
        let august = id(netflix, date(2026, 8, 18));
        unmatch(&mut w.entries, august).unwrap();
        let september = schedule_rows(&w.plans, &w.entries, Some(Period::of(today())), today());
        let carried = september.iter().find(|r| r.id == august).unwrap();
        assert!(carried.carried);
        let own = schedule_rows(
            &w.plans,
            &w.entries,
            Some(Period::of(today()).prev()),
            today(),
        );
        assert!(own.iter().any(|r| r.id == august && !r.carried));
        let summary = period_summary(&september, &w.plans, &w.entries, &w.transactions);
        assert_eq!(summary.overdue, 3);
    }

    #[test]
    fn unresolved_rows_carry_into_any_other_month_once() {
        let w = world();
        let august = schedule_rows(
            &w.plans,
            &w.entries,
            Some(Period::of(today()).prev()),
            today(),
        );
        let actionable: Vec<&ScheduleRow> = august
            .iter()
            .filter(|r| matches!(r.status, BillStatus::Overdue | BillStatus::Due))
            .collect();
        // September's two Overdue and three Due rows all reach back into August, carried.
        assert_eq!(actionable.len(), 5);
        assert!(actionable.iter().all(|r| r.carried));
        let ids: std::collections::HashSet<EntryId> = august.iter().map(|r| r.id).collect();
        assert_eq!(ids.len(), august.len());
    }

    #[test]
    fn all_shows_every_entry_unresolved_first_then_latest_resolved() {
        let w = world();
        let all = schedule_rows(&w.plans, &w.entries, None, today());
        assert_eq!(
            all.len(),
            w.entries.iter().filter(|e| !e.superseded).count()
        );
        assert!(all.iter().all(|r| !r.carried && !r.preview));
        let split = all.iter().position(ScheduleRow::is_resolved).unwrap();
        let (open, settled) = all.split_at(split);
        assert!(settled.iter().all(ScheduleRow::is_resolved));
        assert!(open.windows(2).all(|w| w[0].id.due <= w[1].id.due));
        assert!(settled.windows(2).all(|w| w[0].id.due >= w[1].id.due));
    }

    #[test]
    fn a_carried_entry_is_shown_in_the_current_period_and_leads_needs_attention() {
        let mut w = world();
        let netflix = plan_id(&w, "Netflix");
        let august = id(netflix, date(2026, 8, 18));
        unmatch(&mut w.entries, august).unwrap();
        let carried = entry(&w.entries, august).unwrap();
        assert_eq!(schedule_period(carried, today()), Period::of(today()));
        assert_eq!(attention_entries(&w.plans, &w.entries, today())[0], august);
        let rent = entry(&w.entries, id(plan_id(&w, "Rent"), date(2026, 10, 1))).unwrap();
        assert_eq!(schedule_period(rent, today()), Period::of(today()).next());
    }

    #[test]
    fn a_lead_reaching_into_next_month_brings_its_entry_into_needs_attention() {
        let mut w = world();
        let rent = plan_id(&w, "Rent");
        let october = id(rent, date(2026, 10, 1));
        assert!(!attention_entries(&w.plans, &w.entries, today()).contains(&october));
        w.plans
            .iter_mut()
            .find(|p| p.id == rent)
            .unwrap()
            .attention_lead = Some(31);
        assert!(attention_entries(&w.plans, &w.entries, today()).contains(&october));
    }

    #[test]
    fn periods_past_the_horizon_show_previews() {
        let w = world();
        let january = Period {
            year: 2027,
            month: 1,
        };
        let rows = schedule_rows(&w.plans, &w.entries, Some(january), today());
        assert!(rows.iter().any(|r| r.preview));
        let rent = plan_id(&w, "Rent");
        assert!(rows.iter().any(|r| r.id == id(rent, date(2027, 1, 1))));
        // The inactive Plan never previews.
        let closed = plan_id(&w, "Basic Fitness (closed)");
        assert!(rows.iter().all(|r| r.id.plan_id != closed));
    }

    #[test]
    fn the_period_summary_counts_and_totals_its_own_rows() {
        let w = world();
        let rows = schedule_rows(&w.plans, &w.entries, Some(Period::of(today())), today());
        let summary = period_summary(&rows, &w.plans, &w.entries, &w.transactions);
        assert_eq!((summary.due, summary.overdue, summary.paid), (3, 2, 1));
        // 29.99 + 25.00 + 64.00 + 22.99 + 89.00 + 150.00 + 1180.00
        assert_eq!(
            summary.planned,
            Total::Single {
                unit: "aud".into(),
                amount: cents_money(156_098),
            }
        );
    }

    #[test]
    fn history_past_months_are_settled_by_transactions() {
        let w = world();
        let telstra = plan_id(&w, "Telstra Internet");
        let past: Vec<&BillScheduleEntry> = w
            .entries
            .iter()
            .filter(|e| e.plan_id == telstra && e.due < Period::of(today()).first_day())
            .collect();
        assert_eq!(past.len(), 15);
        assert!(
            past.iter()
                .all(|e| matches!(e.resolution, Resolution::Paid(_)))
        );
        // Same month last year exists for the History figures.
        assert!(past.iter().any(|e| e.due == date(2025, 9, 22)));
        let gym = plan_id(&w, "Gym \u{2014} Fitness First");
        assert!(
            w.entries
                .iter()
                .any(|e| e.plan_id == gym && matches!(e.resolution, Resolution::Skipped { .. }))
        );
        // The closed Plan stopped at its Ends On.
        let closed = plan_id(&w, "Basic Fitness (closed)");
        assert_eq!(dues(&w, closed).last(), Some(&date(2026, 5, 12)));
    }

    #[test]
    fn the_seed_is_deterministic() {
        let first = world();
        let second = world();
        assert_eq!(first.entries, second.entries);
        assert_eq!(first.transactions, second.transactions);
    }
}
