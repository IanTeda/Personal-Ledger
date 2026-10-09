//! Category detail (9d): the Splits behind a Category's Spent in a month, and its track record.

use std::collections::BTreeMap;

use bigdecimal::{BigDecimal, RoundingMode};
use chrono::NaiveDate;
use lib_core::{CategoryTypes, Money, Period};

use lib_categories as categories;

use super::figures::{
    CategoryFigures, Elapsed, LeafFigures, Ledger, SpentIndex, effective, elapsed, known_costs,
    leaf_row, parent_row,
};
use super::{Budget, Rollover, zero};

/// How many closed months before the month shown the track record looks back over.
const TRACK_MONTHS: usize = 6;

/// One Split behind a Category's Spent in a month.
#[derive(Debug, Clone, PartialEq)]
pub struct DetailLine {
    pub date: NaiveDate,
    pub account_id: u32,
    pub payee_id: Option<u32>,
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
    pub category_id: u32,
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
    ledger: &Ledger<'_>,
    category_id: u32,
    month: Period,
    today: NaiveDate,
) -> Vec<u32> {
    let below: Vec<u32> = categories::descendants_inclusive(ledger.categories, category_id)
        .into_iter()
        .filter(|id| {
            categories::is_leaf(ledger.categories, *id)
                && ledger
                    .categories
                    .iter()
                    .any(|c| c.id == *id && c.category_type == CategoryTypes::Expense)
        })
        .collect();
    let budgeted: Vec<u32> = below
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
    ledger: &Ledger<'_>,
    category_id: u32,
    month: Period,
    today: NaiveDate,
) -> Option<CategoryDetail> {
    let category = ledger.categories.iter().find(|c| c.id == category_id)?;
    if category.category_type != CategoryTypes::Expense {
        return None;
    }
    let index = SpentIndex::build(budget, ledger);
    let is_parent = !categories::is_leaf(ledger.categories, category_id);
    let leaves = detail_leaves(budget, &index, ledger, category_id, month, today);
    let figures: BTreeMap<u32, LeafFigures> = leaves
        .iter()
        .map(|id| {
            let (known, known_count) = known_costs(budget, ledger, *id, month, today);
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
        let under: Vec<(&u32, &LeafFigures)> = figures.iter().collect();
        parent_row(category_id, &under)
    } else {
        figures
            .get(&category_id)
            .map(|leaf| leaf_row(category_id, leaf))?
    };

    let mut lines: Vec<DetailLine> = ledger
        .transactions
        .iter()
        .filter(|t| budget.account_ids.contains(&t.account_id) && month.contains(t.date))
        .flat_map(|t| {
            let leaves = &leaves;
            t.splits
                .iter()
                .filter(move |s| leaves.contains(&s.category_id))
                .map(move |s| DetailLine {
                    date: t.date,
                    account_id: t.account_id,
                    payee_id: s.payee_id,
                    amount: Money(-s.amount.0.clone()),
                })
        })
        .collect();
    lines.sort_by(|a, b| b.amount.0.cmp(&a.amount.0).then(b.date.cmp(&a.date)));

    let bill_plans = ledger
        .plans
        .iter()
        .filter(|plan| {
            plan.is_active
                && budget.account_ids.contains(&plan.account_id)
                && categories::descendants_inclusive(ledger.categories, category_id)
                    .contains(&plan.category_id)
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
    leaves: &[u32],
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
