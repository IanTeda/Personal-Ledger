//! The Bill Plan and Schedule rules that act on Accounts, Categories, Payees and Transactions:
//! validation and CRUD for Plans, Pay, Match and Skip, and the Planner's order. Pure and
//! in-memory, the same as the rest of this crate.

use bigdecimal::{BigDecimal, Signed};
use chrono::NaiveDate;
use lib_accounts::Account;
use lib_categories::{self as categories, Category};
use lib_core::{CategoryTypes, Money, TransactionStatus};
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
