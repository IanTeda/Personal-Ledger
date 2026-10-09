//! The Bill Plan and Schedule rules that act on Accounts, Categories, Payees and Transactions:
//! validation and CRUD for Plans, Pay, Match and Skip, and the Planner's order. Pure and
//! in-memory, the same as the rest of this crate.

use bigdecimal::{BigDecimal, Signed};
use chrono::NaiveDate;
use lib_accounts::Account;
use lib_categories::{self as categories, Category};
use lib_core::{CategoryTypes, Money, Period, Total, TransactionStatus};
use lib_transactions::{self as transactions, Split, Transaction};

use super::*;

/// The amount in cents, the Money the Pay and Match rules fold from.
fn cents_money(cents: i64) -> Money {
    Money(BigDecimal::new(cents.into(), 2))
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

#[cfg(test)]
mod tests {
    use super::*;
    use lib_accounts::default_accounts;
    use lib_categories::default_categories;

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

    fn id(plan_id: u32, due: NaiveDate) -> EntryId {
        EntryId { plan_id, due }
    }

    // --- recurrence ---

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

    #[test]
    fn a_one_shot_can_be_skipped_as_cancelled() {
        let mut w = empty();
        let pid = add(&mut w, &draft(Recurrence::OneShot, date(2026, 10, 3)));
        assert_eq!(dues(&w, pid), vec![date(2026, 10, 3)]);
        skip(&w.plans, &mut w.entries, id(pid, date(2026, 10, 3))).unwrap();
        populate(&w.plans, &mut w.entries, today());
        assert_eq!(dues(&w, pid), vec![date(2026, 10, 3)]);
    }
}
