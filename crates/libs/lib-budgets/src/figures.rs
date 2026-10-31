//! The figures every Budgets surface reads for a month: Spent, carries, Known Costs, the
//! Progress rows, the switcher's health line and the Dashboard's bars, all from [`period_figures`].

use std::collections::BTreeMap;

use bigdecimal::{BigDecimal, RoundingMode, Signed, Zero};
use chrono::{Datelike, NaiveDate};
use lib_core::{CategoryId, CategoryTypes, Money, Period};

use super::protocols::{
    BudgetAccount, BudgetBill, BudgetCategories, BudgetCategory, BudgetReads, BudgetSplit,
};
use super::{Budget, LimitRecord, Rollover, applied, cents_money, tree, zero};

/// The Splits on Budget `budget`'s Accounts dated from `first` to `last`, reaching back to the
/// Budget's first record so a carry can walk its whole chain.
pub(super) fn counted_splits(
    budget: &Budget,
    reads: &dyn BudgetReads,
    accounts: &[BudgetAccount],
    first: Period,
    last: Period,
) -> Vec<BudgetSplit> {
    let first = budget
        .first_record_month()
        .map_or(first, |earliest| earliest.min(first));
    reads
        .budget_splits(first.first_day(), last.last_day())
        .into_iter()
        .filter(|split| budget.counts(accounts, split.account_id))
        .collect()
}

/// Spent per (leaf Category, month) on one Budget's Accounts, built once per query so the carry's
/// walk down a chain doesn't rescan the Splits for every month.
pub(super) struct SpentIndex(BTreeMap<(CategoryId, Period), BigDecimal>);

impl SpentIndex {
    /// Indexes `splits`, which [`counted_splits`] has already held to the Budget's Accounts.
    pub(super) fn build(splits: &[BudgetSplit]) -> Self {
        let mut spent: BTreeMap<(CategoryId, Period), BigDecimal> = BTreeMap::new();
        for split in splits {
            *spent
                .entry((split.category_id, Period::of(split.date)))
                .or_default() -= split.amount.0.clone();
        }
        Self(spent)
    }

    /// Loads and indexes what Budget `budget` spent from `first` to `last`.
    pub(super) fn load(
        budget: &Budget,
        reads: &dyn BudgetReads,
        accounts: &[BudgetAccount],
        first: Period,
        last: Period,
    ) -> Self {
        Self::build(&counted_splits(budget, reads, accounts, first, last))
    }

    pub(super) fn spent(&self, category_id: CategoryId, month: Period) -> BigDecimal {
        self.0
            .get(&(category_id, month))
            .cloned()
            .unwrap_or_default()
    }
}

/// A leaf's carry into `month`: the compounded leftover of every closed month before it.
pub(super) fn carry_in(
    chain: &[LimitRecord],
    index: &SpentIndex,
    category_id: CategoryId,
    month: Period,
    today: NaiveDate,
) -> BigDecimal {
    // Month `month - 1` must be closed for anything to carry into `month`.
    if month > Period::of(today) {
        return zero();
    }
    let Some(first) = chain.first().map(|record| record.month) else {
        return zero();
    };
    let mut carry = zero();
    let mut walking = first;
    while walking < month {
        carry = match applied(chain, walking) {
            None => zero(),
            Some(found) => {
                let left = found.amount.0 + carry - index.spent(category_id, walking);
                match found.rollover {
                    Rollover::None => zero(),
                    Rollover::CarryUnspent if left.is_negative() => zero(),
                    Rollover::CarryUnspent | Rollover::CarryBoth => left,
                }
            }
        };
        walking = walking.next();
    }
    carry
}

/// A leaf's budget for a month: the Budget Amount and what carried in.
#[derive(Debug, Clone, PartialEq)]
pub struct Effective {
    pub amount: Money,
    pub carried_in: Money,
    pub rollover: Rollover,
    pub own_record: bool,
}

impl Effective {
    /// Amount plus carry: what Spent is measured against. Negative when an overspend carried.
    pub fn total(&self) -> Money {
        Money(self.amount.0.clone() + self.carried_in.0.clone())
    }
}

pub(super) fn effective(
    budget: &Budget,
    index: &SpentIndex,
    category_id: CategoryId,
    month: Period,
    today: NaiveDate,
) -> Option<Effective> {
    let chain = budget.chain(category_id);
    let found = applied(chain, month)?;
    Some(Effective {
        amount: found.amount,
        carried_in: Money(carry_in(chain, index, category_id, month, today)),
        rollover: found.rollover,
        own_record: found.own_record,
    })
}

/// A leaf's effective budget for a month, `None` when Unbudgeted.
pub fn effective_budget(
    budget: &Budget,
    reads: &dyn BudgetReads,
    category_id: CategoryId,
    month: Period,
    today: NaiveDate,
) -> Option<Effective> {
    let accounts = reads.budget_accounts();
    let index = SpentIndex::load(budget, reads, &accounts, month, month);
    effective(budget, &index, category_id, month, today)
}

/// The open Bill Schedule amounts a leaf carries in `month` on the Budget's Accounts, and how
/// many entries make them up.
pub(super) fn known_costs(
    budget: &Budget,
    accounts: &[BudgetAccount],
    bills: &[BudgetBill],
    category_id: CategoryId,
    month: Period,
    today: NaiveDate,
) -> (BigDecimal, usize) {
    let current = Period::of(today);
    if month < current {
        return (zero(), 0);
    }
    let mut total = zero();
    let mut count = 0;
    for bill in bills {
        if bill.category_id != category_id || !budget.counts(accounts, bill.account_id) {
            continue;
        }
        let due = Period::of(bill.due);
        // The current month also takes the Overdue entries carried in from past months.
        let counts = if month == current {
            due <= current
        } else {
            due == month
        };
        if counts {
            total += bill.amount.0.clone();
            count += 1;
        }
    }
    (total, count)
}

/// One row of the Progress table.
#[derive(Debug, Clone, PartialEq)]
pub struct CategoryFigures {
    pub category_id: CategoryId,
    /// A rollup of its budgeted leaves: no edit action, and never counted as over.
    pub is_parent: bool,
    /// Effective budget (amount plus carry); `None` when Unbudgeted, which for a parent means no
    /// budgeted child.
    pub budget: Option<Money>,
    pub carried_in: Money,
    pub spent: Money,
    pub known: Money,
    /// Budget minus Spent minus Known Costs; `None` when Unbudgeted.
    pub left: Option<Money>,
    /// Spent is over the effective budget. Counted for leaves only.
    pub over: bool,
    /// Known Costs alone push the leaf over; shown, never counted as over.
    pub at_risk: bool,
}

/// How far through a month `today` is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Elapsed {
    /// 0 for a future month, the month's length for a past one.
    pub day: u32,
    pub days_in_month: u32,
    /// Whole percent: 100 for a past month, 0 for a future one.
    pub percent: u32,
    /// Including today; the whole month for a future one, 0 for a past one.
    pub days_left: u32,
}

pub(super) fn elapsed(month: Period, today: NaiveDate) -> Elapsed {
    let days_in_month = month.last_day().day();
    let current = Period::of(today);
    let (day, days_left) = if month < current {
        (days_in_month, 0)
    } else if month > current {
        (0, days_in_month)
    } else {
        (today.day(), days_in_month - today.day() + 1)
    };
    Elapsed {
        day,
        days_in_month,
        percent: (day * 100 + days_in_month / 2) / days_in_month.max(1),
        days_left,
    }
}

/// Everything the Progress tab, the switcher's health line, the rail badge and the Dashboard
/// read for one month.
#[derive(Debug, Clone, PartialEq)]
pub struct PeriodFigures {
    pub month: Period,
    /// Tree order, parents before their children.
    pub rows: Vec<CategoryFigures>,
    /// Budgeted leaves' effective budget (amount plus carry).
    pub budgeted: Money,
    pub carried_in: Money,
    pub spent: Money,
    pub known: Money,
    /// The unpaid bills behind `known`.
    pub known_count: usize,
    /// BUDGETED − SPENT − KNOWN over budgeted leaves; negative when the month is over.
    pub left: Money,
    /// Spent on leaves with no Budget Amount this month.
    pub unbudgeted_spent: Money,
    /// Known Costs on leaves with no Budget Amount this month.
    pub unbudgeted_known: Money,
    pub over_count: usize,
    pub at_risk_count: usize,
    pub elapsed: Elapsed,
    /// Left ÷ days left, when there is something left and days to spend it in.
    pub per_day: Option<Money>,
}

pub(super) struct LeafFigures {
    pub(super) effective: Option<Effective>,
    pub(super) spent: BigDecimal,
    pub(super) known: BigDecimal,
    pub(super) known_count: usize,
}

impl LeafFigures {
    fn over(&self) -> bool {
        self.effective
            .as_ref()
            .is_some_and(|e| self.spent > e.total().0)
    }

    fn at_risk(&self) -> bool {
        self.effective.as_ref().is_some_and(|e| {
            self.spent <= e.total().0 && self.spent.clone() + self.known.clone() > e.total().0
        })
    }
}

pub(super) fn expense_leaves(categories: &[BudgetCategory]) -> Vec<CategoryId> {
    categories
        .iter()
        .filter(|c| c.category_type == CategoryTypes::Expense && c.is_leaf)
        .map(|c| c.id)
        .collect()
}

/// The leaf Expense Categories with no Budget Amount in `month` (never budgeted, or Stopped):
/// what 9e's Category picker offers.
pub fn unbudgeted_leaves(
    budget: &Budget,
    categories: &dyn BudgetCategories,
    month: Period,
) -> Vec<CategoryId> {
    expense_leaves(&categories.budget_categories())
        .into_iter()
        .filter(|id| applied(budget.chain(*id), month).is_none())
        .collect()
}

/// The one shared period query: every figure a Budget shows for `month`, as of `today`.
pub fn period_figures(
    budget: &Budget,
    reads: &dyn BudgetReads,
    month: Period,
    today: NaiveDate,
) -> PeriodFigures {
    let categories = reads.budget_categories();
    let accounts = reads.budget_accounts();
    let bills = reads.open_bills();
    let index = SpentIndex::load(budget, reads, &accounts, month, month);
    let leaves: BTreeMap<CategoryId, LeafFigures> = expense_leaves(&categories)
        .into_iter()
        .map(|id| {
            let (known, known_count) = known_costs(budget, &accounts, &bills, id, month, today);
            let figures = LeafFigures {
                effective: effective(budget, &index, id, month, today),
                spent: index.spent(id, month),
                known,
                known_count,
            };
            (id, figures)
        })
        .collect();

    let mut rows = Vec::new();
    for category in &categories {
        let id = category.id;
        if category.category_type != CategoryTypes::Expense {
            continue;
        }
        if let Some(leaf) = leaves.get(&id) {
            let listed = leaf.effective.is_some() || !leaf.spent.is_zero() || !leaf.known.is_zero();
            if listed {
                rows.push(leaf_row(id, leaf));
            }
            continue;
        }
        let under: Vec<(&CategoryId, &LeafFigures)> = tree::descendants_inclusive(&categories, id)
            .iter()
            .filter_map(|below| leaves.get_key_value(below))
            .collect();
        let any_listed = under
            .iter()
            .any(|(_, l)| l.effective.is_some() || !l.spent.is_zero() || !l.known.is_zero());
        if any_listed {
            rows.push(parent_row(id, &under));
        }
    }

    let budgeted_leaves = || leaves.values().filter(|l| l.effective.is_some());
    let sum = |values: &mut dyn Iterator<Item = BigDecimal>| values.fold(zero(), |a, b| a + b);
    let budgeted =
        sum(&mut budgeted_leaves().filter_map(|l| l.effective.as_ref().map(|e| e.total().0)));
    let carried_in =
        sum(&mut budgeted_leaves()
            .filter_map(|l| l.effective.as_ref().map(|e| e.carried_in.0.clone())));
    let spent = sum(&mut budgeted_leaves().map(|l| l.spent.clone()));
    let known = sum(&mut budgeted_leaves().map(|l| l.known.clone()));
    let known_count = budgeted_leaves().map(|l| l.known_count).sum();
    let unbudgeted = || leaves.values().filter(|l| l.effective.is_none());
    let unbudgeted_spent = sum(&mut unbudgeted().map(|l| l.spent.clone()));
    let unbudgeted_known = sum(&mut unbudgeted().map(|l| l.known.clone()));
    let left = budgeted.clone() - spent.clone() - known.clone();

    let elapsed = elapsed(month, today);
    let per_day = (left.is_positive() && elapsed.days_left > 0).then(|| {
        Money(
            (left.clone() / BigDecimal::from(elapsed.days_left))
                .with_scale_round(2, RoundingMode::HalfUp),
        )
    });

    PeriodFigures {
        month,
        rows,
        budgeted: Money(budgeted),
        carried_in: Money(carried_in),
        spent: Money(spent),
        known: Money(known),
        known_count,
        left: Money(left),
        unbudgeted_spent: Money(unbudgeted_spent),
        unbudgeted_known: Money(unbudgeted_known),
        over_count: leaves.values().filter(|l| l.over()).count(),
        at_risk_count: leaves.values().filter(|l| l.at_risk()).count(),
        elapsed,
        per_day,
    }
}

pub(super) fn leaf_row(category_id: CategoryId, leaf: &LeafFigures) -> CategoryFigures {
    let budget = leaf.effective.as_ref().map(Effective::total);
    let left = budget
        .as_ref()
        .map(|b| Money(b.0.clone() - leaf.spent.clone() - leaf.known.clone()));
    CategoryFigures {
        category_id,
        is_parent: false,
        budget,
        carried_in: leaf
            .effective
            .as_ref()
            .map_or_else(|| cents_money(0), |e| e.carried_in.clone()),
        spent: Money(leaf.spent.clone()),
        known: Money(leaf.known.clone()),
        left,
        over: leaf.over(),
        at_risk: leaf.at_risk(),
    }
}

/// A parent rolls up its budgeted leaves only: an unbudgeted child is listed on its own row and
/// never inflates the parent's Spent.
pub(super) fn parent_row(
    category_id: CategoryId,
    under: &[(&CategoryId, &LeafFigures)],
) -> CategoryFigures {
    let budgeted: Vec<&LeafFigures> = under
        .iter()
        .map(|(_, l)| *l)
        .filter(|l| l.effective.is_some())
        .collect();
    let has_budget = !budgeted.is_empty();
    let total = budgeted
        .iter()
        .filter_map(|l| l.effective.as_ref())
        .fold(zero(), |a, e| a + e.total().0);
    let carried_in = budgeted
        .iter()
        .filter_map(|l| l.effective.as_ref())
        .fold(zero(), |a, e| a + e.carried_in.0.clone());
    let spent = budgeted.iter().fold(zero(), |a, l| a + l.spent.clone());
    let known = budgeted.iter().fold(zero(), |a, l| a + l.known.clone());
    CategoryFigures {
        category_id,
        is_parent: true,
        budget: has_budget.then(|| Money(total.clone())),
        carried_in: Money(carried_in),
        over: has_budget && spent > total,
        at_risk: has_budget && spent <= total && spent.clone() + known.clone() > total,
        left: has_budget.then(|| Money(total - spent.clone() - known.clone())),
        spent: Money(spent),
        known: Money(known),
    }
}

/// The switcher's health line for a Limits Budget: N over · X left to spend (· M at risk).
#[derive(Debug, Clone, PartialEq)]
pub struct Health {
    pub over: usize,
    /// Effective budget minus Spent minus Known Costs over budgeted leaves; can be negative.
    pub left: Money,
    pub at_risk: usize,
}

pub fn health(budget: &Budget, reads: &dyn BudgetReads, today: NaiveDate) -> Health {
    let figures = period_figures(budget, reads, Period::of(today), today);
    Health {
        over: figures.over_count,
        left: figures.left,
        at_risk: figures.at_risk_count,
    }
}

/// How many bars the Dashboard's budget list shows.
pub const DASHBOARD_BARS: usize = 4;

/// One bar of the Dashboard's budget list.
#[derive(Debug, Clone, PartialEq)]
pub struct DashboardBar {
    pub category_id: CategoryId,
    pub budget: Money,
    pub spent: Money,
    pub over: bool,
}

/// The Dashboard's budget list: the budgeted top-level rows (a leaf, or a parent's rollup) that
/// have used the largest share of their budget, [`DASHBOARD_BARS`] at most.
pub fn dashboard_bars(
    figures: &PeriodFigures,
    categories: &dyn BudgetCategories,
) -> Vec<DashboardBar> {
    let categories = categories.budget_categories();
    let mut bars: Vec<DashboardBar> = figures
        .rows
        .iter()
        .filter(|row| tree::get(&categories, row.category_id).is_none_or(|c| c.parent.is_none()))
        .filter_map(|row| {
            let budget = row.budget.clone()?;
            Some(DashboardBar {
                category_id: row.category_id,
                over: row.spent.0 > budget.0,
                spent: row.spent.clone(),
                budget,
            })
        })
        .collect();
    // Shares compared by cross-multiplying, so a 0.00 budget needs no division. The sort is
    // stable: equal shares keep their tree order.
    bars.sort_by(|a, b| {
        (b.spent.0.clone() * a.budget.0.clone()).cmp(&(a.spent.0.clone() * b.budget.0.clone()))
    });
    bars.truncate(DASHBOARD_BARS);
    bars
}
