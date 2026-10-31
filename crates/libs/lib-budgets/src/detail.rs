//! Category detail (9d): the Splits behind a Category's Spent in a month, and its track record.

use std::collections::BTreeMap;

use bigdecimal::{BigDecimal, RoundingMode};
use chrono::NaiveDate;
use lib_core::{AccountId, CategoryId, Money, PayeeId, Period};

use super::figures::{
    CategoryFigures, Elapsed, LeafFigures, SpentIndex, counted_splits, effective, elapsed,
    known_costs, leaf_row, parent_row,
};
use super::protocols::{BudgetCategory, BudgetReads};
use super::{Budget, Rollover, tree, zero};

/// How many closed months before the month shown the track record looks back over.
const TRACK_MONTHS: usize = 6;

/// One Split behind a Category's Spent in a month.
#[derive(Debug, Clone, PartialEq)]
pub struct DetailLine {
    pub date: NaiveDate,
    pub account_id: AccountId,
    pub payee_id: Option<PayeeId>,
    /// Signed like Spent: positive when money went out.
    pub amount: Money,
}

/// How often a Category went over in the closed months before the one shown.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackRecord {
    /// Closed, budgeted months looked at (at most [`TRACK_MONTHS`]).
    pub months: u32,
    pub over_months: u32,
    /// Mean Spent over those months.
    pub average: Money,
}

/// Everything 9d shows for a Category (a leaf or a parent's rollup) and month.
#[derive(Debug, Clone, PartialEq)]
pub struct CategoryDetail {
    pub category_id: CategoryId,
    pub month: Period,
    /// The same figures as the Progress row.
    pub figures: CategoryFigures,
    pub elapsed: Elapsed,
    /// Every Split behind Spent, largest first.
    pub lines: Vec<DetailLine>,
    /// Active Bill Plans in the Category on the Budget's Accounts.
    pub bill_plans: usize,
    /// `None` when the Category was budgeted in none of the months before this one.
    pub track: Option<TrackRecord>,
    /// The leaf's Rollover; a parent's rollup has none of its own.
    pub rollover: Option<Rollover>,
}

/// The leaves a Category's figures cover: its budgeted leaves, or every leaf below it when none
/// is budgeted this month (an unbudgeted row still lists what was spent).
fn detail_leaves(
    budget: &Budget,
    index: &SpentIndex,
    categories: &[BudgetCategory],
    category_id: CategoryId,
    month: Period,
    today: NaiveDate,
) -> Vec<CategoryId> {
    let below: Vec<CategoryId> = tree::descendants_inclusive(categories, category_id)
        .into_iter()
        .filter(|id| tree::is_leaf(categories, *id) && tree::is_expense(categories, *id))
        .collect();
    let budgeted: Vec<CategoryId> = below
        .iter()
        .copied()
        .filter(|id| effective(budget, index, *id, month, today).is_some())
        .collect();
    if budgeted.is_empty() { below } else { budgeted }
}

/// 9d's figures for `category_id` in `month`, as of `today`. `None` for a Category that is not an
/// Expense Category.
pub fn category_detail(
    budget: &Budget,
    reads: &dyn BudgetReads,
    category_id: CategoryId,
    month: Period,
    today: NaiveDate,
) -> Option<CategoryDetail> {
    let categories = reads.budget_categories();
    if !tree::is_expense(&categories, category_id) {
        return None;
    }
    let accounts = reads.budget_accounts();
    let bills = reads.open_bills();
    let splits = counted_splits(budget, reads, &accounts, month, month);
    let index = SpentIndex::build(&splits);
    let is_parent = !tree::is_leaf(&categories, category_id);
    let leaves = detail_leaves(budget, &index, &categories, category_id, month, today);
    let figures: BTreeMap<CategoryId, LeafFigures> = leaves
        .iter()
        .map(|id| {
            let (known, known_count) = known_costs(budget, &accounts, &bills, *id, month, today);
            let leaf = LeafFigures {
                effective: effective(budget, &index, *id, month, today),
                spent: index.spent(*id, month),
                known,
                known_count,
            };
            (*id, leaf)
        })
        .collect();
    let row = if is_parent {
        let under: Vec<(&CategoryId, &LeafFigures)> = figures.iter().collect();
        parent_row(category_id, &under)
    } else {
        figures
            .get(&category_id)
            .map(|leaf| leaf_row(category_id, leaf))?
    };

    let mut lines: Vec<DetailLine> = splits
        .iter()
        .filter(|s| month.contains(s.date) && leaves.contains(&s.category_id))
        .map(|s| DetailLine {
            date: s.date,
            account_id: s.account_id,
            payee_id: s.payee_id,
            amount: Money(-s.amount.0.clone()),
        })
        .collect();
    lines.sort_by(|a, b| b.amount.0.cmp(&a.amount.0).then(b.date.cmp(&a.date)));

    let below = tree::descendants_inclusive(&categories, category_id);
    let bill_plans = reads
        .active_bill_plans()
        .iter()
        .filter(|plan| {
            budget.counts(&accounts, plan.account_id) && below.contains(&plan.category_id)
        })
        .count();

    Some(CategoryDetail {
        category_id,
        month,
        figures: row,
        elapsed: elapsed(month, today),
        lines,
        bill_plans,
        track: track_record(budget, &index, &leaves, month, today),
        rollover: if is_parent {
            None
        } else {
            effective(budget, &index, category_id, month, today).map(|e| e.rollover)
        },
    })
}

/// Over-months and mean Spent across the (up to six) closed months before `month` in which any of
/// `leaves` was budgeted. A month counts once: it is over when the budgeted leaves' Spent together
/// exceeds their effective budgets together, the parent rollup's own rule.
fn track_record(
    budget: &Budget,
    index: &SpentIndex,
    leaves: &[CategoryId],
    month: Period,
    today: NaiveDate,
) -> Option<TrackRecord> {
    let current = Period::of(today);
    let first = budget.first_amount_month()?;
    let mut walking = month;
    let mut months = 0_u32;
    let mut over_months = 0_u32;
    let mut spent_total = zero();
    while months < u32::try_from(TRACK_MONTHS).unwrap_or(u32::MAX) {
        walking = walking.prev();
        if walking < first {
            break;
        }
        if walking >= current {
            continue;
        }
        let (mut budgeted, mut spent, mut any) = (zero(), zero(), false);
        for id in leaves {
            if let Some(effective) = effective(budget, index, *id, walking, today) {
                any = true;
                budgeted += effective.total().0;
                spent += index.spent(*id, walking);
            }
        }
        if !any {
            continue;
        }
        months += 1;
        if spent > budgeted {
            over_months += 1;
        }
        spent_total += spent;
    }
    (months > 0).then(|| TrackRecord {
        months,
        over_months,
        average: Money(
            (spent_total / BigDecimal::from(months)).with_scale_round(2, RoundingMode::HalfUp),
        ),
    })
}
