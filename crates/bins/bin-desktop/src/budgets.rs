//! Pure Budgets-surface domain types and the stub dataset behind them (`docs/ux/desktop/14-budgets-v2/`,
//! ADR-0028 and ADR-0029) -- `gpui`-free and in-memory, the same "pure state, chrome renders it"
//! split `bills.rs` uses. Nothing here reads `lib_database`.
//!
//! The rules are the Desktop Budgets Surface map's settled decisions (#383–#387, #399, #400):
//!
//! - A [`Budget`] is a named container: a [`Method`], one Unit locked at creation, the on-budget
//!   Accounts, a default flag and an archived date. It owns a chain of [`LimitRecord`]s per leaf
//!   Expense Category and never owns Transactions, so Budgets may overlap.
//! - A chain holds at most one record per month: **Onward** (from its month until the next record),
//!   **Month-only** (its month alone, the next month falls back to the Onward value carried
//!   forward) and **Stop** (no amount from its month). No amount is Unbudgeted; 0.00 is budgeted.
//! - **Rollover** is held on each record. A closed month's carry is its effective budget (amount
//!   plus carry in) minus Spent, compounds uncapped, and resets on a Stop or an unbudgeted month.
//!   The current month never carries, and carries are computed on read, never stored.
//! - **Spent** is signed (a refund nets off), positive when money went out, and counts every
//!   Transaction Status on the Budget's on-budget Accounts. **Known Costs** are the open Bill
//!   Schedule entries on those Accounts: the current month's plus carried-in Overdue, none for a
//!   past month, and a future month's own.
//! - Every figure on every surface comes from [`period_figures`], [`history`] and their
//!   helpers, all pure over an injectable `today`, so nothing is counted twice.
//!
//! Category ids and Account ids are the stubs' `u32`s, like every other desktop stub.

use std::collections::BTreeMap;

use bigdecimal::{BigDecimal, RoundingMode, Signed, Zero};
use chrono::{Datelike, NaiveDate};
use lib_core::{CategoryTypes, Money};

use crate::{
    accounts::{self, Account},
    bills::{BillPlan, BillScheduleEntry},
    budget_form::BudgetForm,
    categories::{self, Category},
    dialog_host::{Dialog, DialogKey, DialogOutcome},
    field::TextField,
    limit_form::{LimitForm, StopForm},
    period::Period,
    transactions::{self, Transaction},
};

/// The id the seeded **Personal spending** Budget carries: Categories 5c binds it by id, so a
/// rename changes nothing (ADR-0029).
pub const PERSONAL_SPENDING_ID: u32 = 1;

/// How far back "the last 3 months" reaches, for the average and Unallocated figures.
const AVERAGE_MONTHS: i32 = 3;

/// How a Budget measures spending. Only Category limits is built; Envelope, Percentage split and
/// Project are named Methods deferred to their own maps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Limits,
}

impl Method {
    pub const ALL: [Method; 1] = [Method::Limits];
}

/// What a Budget Amount carries into the next month once its own month has closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Rollover {
    #[default]
    None,
    /// Only a positive leftover.
    CarryUnspent,
    /// The leftover or the overspend, so the next month's budget can go negative.
    CarryBoth,
}

impl Rollover {
    /// The Plan grid's `r` key: `—` → carry unspent → carry both → `—`.
    pub fn next(self) -> Self {
        match self {
            Rollover::None => Rollover::CarryUnspent,
            Rollover::CarryUnspent => Rollover::CarryBoth,
            Rollover::CarryBoth => Rollover::None,
        }
    }
}

/// What one record of a chain says.
#[derive(Debug, Clone, PartialEq)]
pub enum Limit {
    Onward { amount: Money, rollover: Rollover },
    MonthOnly { amount: Money, rollover: Rollover },
    Stop,
}

/// A chain entry: what starts in `month`.
#[derive(Debug, Clone, PartialEq)]
pub struct LimitRecord {
    pub month: Period,
    pub limit: Limit,
}

/// A named container of Category Limits over one Ledger (glossary: Budget).
#[derive(Debug, Clone, PartialEq)]
pub struct Budget {
    pub id: u32,
    pub name: String,
    pub method: Method,
    /// The Unit code (e.g. `"aud"`), locked at creation; every on-budget Account shares it.
    pub unit: String,
    /// The [`Account::id`]s whose Splits this Budget counts.
    pub account_ids: Vec<u32>,
    pub is_default: bool,
    /// Archived is computed and viewable but read-only; `None` while active.
    pub archived_at: Option<NaiveDate>,
    /// One chain per leaf Expense [`Category::id`], sorted by month with one record per month.
    limits: BTreeMap<u32, Vec<LimitRecord>>,
}

impl Budget {
    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }

    /// A Category's chain, oldest record first.
    pub fn chain(&self, category_id: u32) -> &[LimitRecord] {
        self.limits.get(&category_id).map_or(&[], Vec::as_slice)
    }

    /// The Categories that have (or once had) a chain here.
    pub fn category_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.limits.keys().copied()
    }

    /// The first month a Budget Amount starts in: the History range's left edge.
    pub fn first_amount_month(&self) -> Option<Period> {
        self.limits
            .values()
            .flatten()
            .filter(|record| !matches!(record.limit, Limit::Stop))
            .map(|record| record.month)
            .min()
    }
}

/// The Budget Amount a month resolves to.
#[derive(Debug, Clone, PartialEq)]
pub struct Applied {
    pub amount: Money,
    pub rollover: Rollover,
    /// A record starts in this month: the Plan grid's bold cells (an Onward change or a
    /// Month-only amount).
    pub own_record: bool,
}

/// What `month` resolves to along a chain, or `None` when it is Unbudgeted.
pub fn applied(chain: &[LimitRecord], month: Period) -> Option<Applied> {
    if let Some(record) = chain.iter().find(|record| record.month == month) {
        return match &record.limit {
            Limit::Onward { amount, rollover } | Limit::MonthOnly { amount, rollover } => {
                Some(Applied {
                    amount: amount.clone(),
                    rollover: *rollover,
                    own_record: true,
                })
            }
            Limit::Stop => None,
        };
    }
    // Month-only records never carry forward, so only the latest Onward or Stop before the month
    // decides what falls through.
    let earlier = chain
        .iter()
        .rfind(|record| record.month < month && !matches!(record.limit, Limit::MonthOnly { .. }))?;
    match &earlier.limit {
        Limit::Onward { amount, rollover } => Some(Applied {
            amount: amount.clone(),
            rollover: *rollover,
            own_record: false,
        }),
        Limit::MonthOnly { .. } | Limit::Stop => None,
    }
}

/// The Rollover a new record inherits: the one on the latest record before `month`, else None.
fn inherited_rollover(chain: &[LimitRecord], month: Period) -> Rollover {
    match chain.iter().rfind(|record| record.month < month) {
        Some(LimitRecord {
            limit: Limit::Onward { rollover, .. } | Limit::MonthOnly { rollover, .. },
            ..
        }) => *rollover,
        _ => Rollover::None,
    }
}

fn put(chain: &mut Vec<LimitRecord>, record: LimitRecord) {
    chain.retain(|existing| existing.month != record.month);
    chain.push(record);
    chain.sort_by_key(|record| record.month);
}

fn zero() -> BigDecimal {
    BigDecimal::from(0)
}

/// Builds an exact `Money` from a signed count of cents: `-1850` is `-18.50`.
fn cents_money(cents: i64) -> Money {
    Money(BigDecimal::new(cents.into(), 2))
}

/// The stubs a Budget's figures read: the shared Categories, Accounts, Transactions and Bills.
#[derive(Debug, Clone, Copy)]
pub struct Ledger<'a> {
    pub categories: &'a [Category],
    pub accounts: &'a [Account],
    pub transactions: &'a [Transaction],
    pub plans: &'a [BillPlan],
    pub entries: &'a [BillScheduleEntry],
}

/// Spent per (leaf Category, month) on one Budget's Accounts, built once per query so the carry's
/// walk down a chain doesn't rescan the Transactions for every month.
struct SpentIndex(BTreeMap<(u32, Period), BigDecimal>);

impl SpentIndex {
    fn build(budget: &Budget, ledger: &Ledger<'_>) -> Self {
        let mut spent: BTreeMap<(u32, Period), BigDecimal> = BTreeMap::new();
        for transaction in ledger
            .transactions
            .iter()
            .filter(|t| budget.account_ids.contains(&t.account_id))
        {
            let month = Period::of(transaction.date);
            for split in &transaction.splits {
                *spent.entry((split.category_id, month)).or_default() -= split.amount.0.clone();
            }
        }
        Self(spent)
    }

    fn spent(&self, category_id: u32, month: Period) -> BigDecimal {
        self.0
            .get(&(category_id, month))
            .cloned()
            .unwrap_or_default()
    }
}

/// A leaf's carry into `month`: the compounded leftover of every closed month before it.
fn carry_in(
    chain: &[LimitRecord],
    index: &SpentIndex,
    category_id: u32,
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

fn effective(
    budget: &Budget,
    index: &SpentIndex,
    category_id: u32,
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
    ledger: &Ledger<'_>,
    category_id: u32,
    month: Period,
    today: NaiveDate,
) -> Option<Effective> {
    effective(
        budget,
        &SpentIndex::build(budget, ledger),
        category_id,
        month,
        today,
    )
}

/// The open Bill Schedule amounts a leaf carries in `month` on the Budget's Accounts, and how
/// many entries make them up.
fn known_costs(
    budget: &Budget,
    ledger: &Ledger<'_>,
    category_id: u32,
    month: Period,
    today: NaiveDate,
) -> (BigDecimal, usize) {
    let current = Period::of(today);
    if month < current {
        return (zero(), 0);
    }
    let mut total = zero();
    let mut count = 0;
    for entry in ledger.entries.iter().filter(|entry| entry.is_open()) {
        let Some(plan) = crate::bills::get(ledger.plans, entry.plan_id) else {
            continue;
        };
        if plan.category_id != category_id || !budget.account_ids.contains(&plan.account_id) {
            continue;
        }
        let due = Period::of(entry.due);
        // The current month also takes the Overdue entries carried in from past months.
        let counts = if month == current {
            due <= current
        } else {
            due == month
        };
        if counts {
            total += plan.planned_amount.0.clone();
            count += 1;
        }
    }
    (total, count)
}

/// One row of the Progress table.
#[derive(Debug, Clone, PartialEq)]
pub struct CategoryFigures {
    pub category_id: u32,
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

fn elapsed(month: Period, today: NaiveDate) -> Elapsed {
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

struct LeafFigures {
    effective: Option<Effective>,
    spent: BigDecimal,
    known: BigDecimal,
    known_count: usize,
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

fn expense_leaves(categories: &[Category]) -> Vec<u32> {
    categories
        .iter()
        .filter(|c| {
            c.category_type == CategoryTypes::Expense && categories::is_leaf(categories, c.id)
        })
        .map(|c| c.id)
        .collect()
}

/// The leaf Expense Categories with no Budget Amount in `month` (never budgeted, or Stopped):
/// what 9e's Category picker offers.
pub fn unbudgeted_leaves(budget: &Budget, categories: &[Category], month: Period) -> Vec<u32> {
    expense_leaves(categories)
        .into_iter()
        .filter(|id| applied(budget.chain(*id), month).is_none())
        .collect()
}

/// The one shared period query: every figure a Budget shows for `month`, as of `today`.
pub fn period_figures(
    budget: &Budget,
    ledger: &Ledger<'_>,
    month: Period,
    today: NaiveDate,
) -> PeriodFigures {
    let index = SpentIndex::build(budget, ledger);
    let leaves: BTreeMap<u32, LeafFigures> = expense_leaves(ledger.categories)
        .into_iter()
        .map(|id| {
            let (known, known_count) = known_costs(budget, ledger, id, month, today);
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
    for (id, _) in categories::paths_in_tree_order(ledger.categories) {
        let Some(category) = ledger.categories.iter().find(|c| c.id == id) else {
            continue;
        };
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
        let under: Vec<(&u32, &LeafFigures)> =
            categories::descendants_inclusive(ledger.categories, id)
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

fn leaf_row(category_id: u32, leaf: &LeafFigures) -> CategoryFigures {
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
fn parent_row(category_id: u32, under: &[(&u32, &LeafFigures)]) -> CategoryFigures {
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

pub fn health(budget: &Budget, ledger: &Ledger<'_>, today: NaiveDate) -> Health {
    let figures = period_figures(budget, ledger, Period::of(today), today);
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
    pub category_id: u32,
    pub budget: Money,
    pub spent: Money,
    pub over: bool,
}

/// The Dashboard's budget list: the budgeted top-level rows (a leaf, or a parent's rollup) that
/// have used the largest share of their budget, [`DASHBOARD_BARS`] at most.
pub fn dashboard_bars(figures: &PeriodFigures, categories: &[Category]) -> Vec<DashboardBar> {
    let mut bars: Vec<DashboardBar> = figures
        .rows
        .iter()
        .filter(|row| categories::depth(categories, row.category_id) == 0)
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

// ---------------------------------------------------------------------------------------------
// Category detail (9d)
// ---------------------------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------------------------
// History
// ---------------------------------------------------------------------------------------------

/// One month of one History row: what applied then against what was spent.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryCell {
    pub budget: Money,
    pub spent: Money,
    pub over: bool,
}

/// A History row: a budgeted leaf or a parent's rollup, one cell per visible month (`None` is the
/// `—` of an unbudgeted month).
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryRow {
    pub category_id: u32,
    pub is_parent: bool,
    pub cells: Vec<Option<HistoryCell>>,
    /// Mean Spent over the closed months shown in which the row was budgeted.
    pub average: Option<Money>,
    /// Closed budgeted months that were over.
    pub over_months: u32,
    /// Closed months shown in which the row was budgeted: OVER's denominator.
    pub closed_months: u32,
}

/// The chart's pair for one month: totals over budgeted leaves only.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryMonth {
    pub month: Period,
    pub budget: Money,
    pub spent: Money,
    /// Total Spent exceeds the total effective budget.
    pub over: bool,
    /// The current month ("to date"), counted in neither AVG nor OVER.
    pub is_current: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct History {
    pub months: Vec<HistoryMonth>,
    pub rows: Vec<HistoryRow>,
    /// Leaves shown out of leaves ever budgeted in this Budget: the status line.
    pub leaves_shown: usize,
    pub leaves_ever_budgeted: usize,
}

/// The range nav's bounds: the first Budget Amount month to the current month, which is the
/// rightmost column (future months belong to Plan).
pub fn history_bounds(budget: &Budget, today: NaiveDate) -> Option<(Period, Period)> {
    budget
        .first_amount_month()
        .map(|first| (first, Period::of(today)))
}

/// Budget against Spent for every month in `first..=last` (both inclusive), each recomputed from
/// the Budget's current Accounts and the current Category tree. Known Costs are left out.
pub fn history(
    budget: &Budget,
    ledger: &Ledger<'_>,
    first: Period,
    last: Period,
    today: NaiveDate,
) -> History {
    let index = SpentIndex::build(budget, ledger);
    let current = Period::of(today);
    let mut months = Vec::new();
    let mut walking = first;
    while walking <= last {
        months.push(walking);
        walking = walking.next();
    }

    // month index -> leaf -> cell
    let leaf_cells: BTreeMap<u32, Vec<Option<HistoryCell>>> = expense_leaves(ledger.categories)
        .into_iter()
        .map(|id| {
            let cells = months
                .iter()
                .map(|&month| {
                    effective(budget, &index, id, month, today).map(|found| {
                        let total = found.total();
                        let spent = index.spent(id, month);
                        HistoryCell {
                            over: spent > total.0,
                            budget: total,
                            spent: Money(spent),
                        }
                    })
                })
                .collect();
            (id, cells)
        })
        .collect();

    let month_totals = months
        .iter()
        .enumerate()
        .map(|(at, &month)| {
            let (budgeted, spent) = leaf_cells
                .values()
                .filter_map(|cells| cells[at].as_ref())
                .fold((zero(), zero()), |(b, s), cell| {
                    (b + cell.budget.0.clone(), s + cell.spent.0.clone())
                });
            HistoryMonth {
                month,
                over: spent > budgeted,
                budget: Money(budgeted),
                spent: Money(spent),
                is_current: month == current,
            }
        })
        .collect();

    let shown = |cells: &[Option<HistoryCell>]| cells.iter().any(Option::is_some);
    let mut rows = Vec::new();
    for (id, _) in categories::paths_in_tree_order(ledger.categories) {
        if let Some(cells) = leaf_cells.get(&id) {
            if shown(cells) {
                rows.push(history_row(id, false, cells.clone(), &months, current));
            }
            continue;
        }
        let Some(category) = ledger.categories.iter().find(|c| c.id == id) else {
            continue;
        };
        if category.category_type != CategoryTypes::Expense {
            continue;
        }
        let below: Vec<&Vec<Option<HistoryCell>>> =
            categories::descendants_inclusive(ledger.categories, id)
                .iter()
                .filter_map(|leaf| leaf_cells.get(leaf))
                .filter(|cells| shown(cells))
                .collect();
        if below.is_empty() {
            continue;
        }
        let rolled: Vec<Option<HistoryCell>> = (0..months.len())
            .map(|at| {
                let cells: Vec<&HistoryCell> = below
                    .iter()
                    .filter_map(|cells| cells[at].as_ref())
                    .collect();
                (!cells.is_empty()).then(|| {
                    let budget = cells.iter().fold(zero(), |a, c| a + c.budget.0.clone());
                    let spent = cells.iter().fold(zero(), |a, c| a + c.spent.0.clone());
                    HistoryCell {
                        over: spent > budget,
                        budget: Money(budget),
                        spent: Money(spent),
                    }
                })
            })
            .collect();
        rows.push(history_row(id, true, rolled, &months, current));
    }

    let leaves_shown = rows.iter().filter(|row| !row.is_parent).count();
    let leaves_ever_budgeted = budget
        .limits
        .iter()
        .filter(|(id, chain)| {
            leaf_cells.contains_key(id) && chain.iter().any(|r| !matches!(r.limit, Limit::Stop))
        })
        .count();
    History {
        months: month_totals,
        rows,
        leaves_shown,
        leaves_ever_budgeted,
    }
}

fn history_row(
    category_id: u32,
    is_parent: bool,
    cells: Vec<Option<HistoryCell>>,
    months: &[Period],
    current: Period,
) -> HistoryRow {
    let closed: Vec<&HistoryCell> = months
        .iter()
        .zip(&cells)
        .filter(|(month, _)| **month < current)
        .filter_map(|(_, cell)| cell.as_ref())
        .collect();
    let closed_months = u32::try_from(closed.len()).unwrap_or(u32::MAX);
    let average = (!closed.is_empty()).then(|| {
        let total = closed.iter().fold(zero(), |a, c| a + c.spent.0.clone());
        Money((total / BigDecimal::from(closed_months)).with_scale_round(2, RoundingMode::HalfUp))
    });
    let over_months = u32::try_from(closed.iter().filter(|c| c.over).count()).unwrap_or(u32::MAX);
    HistoryRow {
        category_id,
        is_parent,
        cells,
        average,
        over_months,
        closed_months,
    }
}

/// How many months the History tab shows at once.
pub const HISTORY_MONTHS: usize = 6;

/// The History window ending at `end`: up to [`HISTORY_MONTHS`] months, held inside
/// [`history_bounds`], or `None` while the Budget has no Budget Amount yet.
pub fn history_range(budget: &Budget, end: Period, today: NaiveDate) -> Option<(Period, Period)> {
    let (earliest, latest) = history_bounds(budget, today)?;
    // A window never ends before it could be full, unless the Budget is younger than that.
    let first_full = (1..HISTORY_MONTHS).fold(earliest, |month, _| month.next());
    let last = end.max(first_full).min(latest);
    let first = (1..HISTORY_MONTHS)
        .fold(last, |month, _| month.prev())
        .max(earliest);
    Some((first, last))
}

/// The History table's column headings for the export, already worded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryCsvLabels {
    pub category: String,
    pub budget: String,
    pub spent: String,
    pub average: String,
    pub over: String,
}

fn csv_field(text: &str) -> String {
    if text.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_string()
    }
}

/// The visible History table as CSV: a row per Category (by path), a budget and a spent column
/// per month, then AVG and OVER. Plain numbers, an unbudgeted month left empty.
pub fn history_csv(
    history: &History,
    categories: &[Category],
    labels: &HistoryCsvLabels,
    month_label: impl Fn(Period) -> String,
) -> String {
    let mut lines = Vec::with_capacity(history.rows.len() + 1);
    let mut header = vec![csv_field(&labels.category)];
    for month in &history.months {
        let month = month_label(month.month);
        header.push(csv_field(&format!("{month} {}", labels.budget)));
        header.push(csv_field(&format!("{month} {}", labels.spent)));
    }
    header.push(csv_field(&labels.average));
    header.push(csv_field(&labels.over));
    lines.push(header.join(","));
    for row in &history.rows {
        let name = categories::path(categories, row.category_id).unwrap_or_default();
        let mut fields = vec![csv_field(&name)];
        for cell in &row.cells {
            match cell {
                Some(cell) => {
                    fields.push(cell.budget.0.with_scale(2).to_string());
                    fields.push(cell.spent.0.with_scale(2).to_string());
                }
                None => fields.extend([String::new(), String::new()]),
            }
        }
        fields.push(
            row.average
                .as_ref()
                .map(|average| average.0.with_scale(2).to_string())
                .unwrap_or_default(),
        );
        fields.push(if row.closed_months == 0 {
            String::new()
        } else {
            format!("{}/{}", row.over_months, row.closed_months)
        });
        lines.push(fields.join(","));
    }
    lines.join("\n") + "\n"
}

// ---------------------------------------------------------------------------------------------
// Plan figures
// ---------------------------------------------------------------------------------------------

/// A month's Total budgeted: Budget Amounts only, no carry.
pub fn total_budgeted(budget: &Budget, categories: &[Category], month: Period) -> Money {
    Money(
        expense_leaves(categories)
            .into_iter()
            .filter_map(|id| applied(budget.chain(id), month))
            .fold(zero(), |sum, found| sum + found.amount.0),
    )
}

/// The three months immediately before `target`.
fn months_before(target: Period) -> impl Iterator<Item = Period> {
    (1..=AVERAGE_MONTHS).map(move |back| (0..back).fold(target, |month, _| month.prev()))
}

fn average_of(index: &SpentIndex, category_id: u32, target: Period) -> Money {
    let total =
        months_before(target).fold(zero(), |sum, month| sum + index.spent(category_id, month));
    let mean = total / BigDecimal::from(AVERAGE_MONTHS);
    // Nearest 10, ties away from zero, and never below zero.
    let rounded = (mean / BigDecimal::from(10)).with_scale_round(0, RoundingMode::HalfUp)
        * BigDecimal::from(10);
    Money(
        if rounded.is_negative() {
            zero()
        } else {
            rounded
        }
        .with_scale(2),
    )
}

/// A leaf's 3-month average Spent on the Budget's Accounts, from the three months before
/// `target`: Fill's source and 11c's "Last 3 months' spending".
pub fn three_month_average(
    budget: &Budget,
    ledger: &Ledger<'_>,
    category_id: u32,
    target: Period,
) -> Money {
    average_of(&SpentIndex::build(budget, ledger), category_id, target)
}

/// Every leaf Expense Category's 3-month average for `target`, tree order.
pub fn three_month_averages(
    budget: &Budget,
    ledger: &Ledger<'_>,
    target: Period,
) -> Vec<(u32, Money)> {
    let index = SpentIndex::build(budget, ledger);
    expense_leaves(ledger.categories)
        .into_iter()
        .map(|id| (id, average_of(&index, id, target)))
        .collect()
}

/// Where Fill (9f) takes a month's amounts from. "The plan" is not a source: it is what the month
/// already resolves to, and so the baseline the preview diffs against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FillSource {
    /// The month before, as budgeted: its Budget Amounts, never its carry.
    #[default]
    PreviousMonth,
    /// The 3-month average Spent before the target.
    Average,
}

impl FillSource {
    pub const ALL: [FillSource; 2] = [FillSource::PreviousMonth, FillSource::Average];
}

/// One Category Fill would change: what the plan gives the month now, and what Fill writes.
#[derive(Debug, Clone, PartialEq)]
pub struct FillLine {
    pub category_id: u32,
    /// `None` when the plan leaves the month Unbudgeted.
    pub before: Option<Money>,
    pub after: Money,
}

/// What one source would do to the target month.
#[derive(Debug, Clone, PartialEq)]
pub struct FillPreview {
    pub target: Period,
    pub source: FillSource,
    /// The Categories whose amount changes, tree order.
    pub changes: Vec<FillLine>,
    /// Categories with their own record in the target month: never overwritten.
    pub kept: Vec<u32>,
    /// Eligible Categories the source leaves at the plan's amount.
    pub unchanged: usize,
    /// The target month's Total budgeted once filled.
    pub total: Money,
}

/// Fill's target: the first open month at or after `cursor`, which is next month when the cursor
/// sits on a closed month (or on no month at all).
pub fn fill_target(cursor: Option<Period>, today: NaiveDate) -> Period {
    let current = Period::of(today);
    match cursor {
        Some(month) if month >= current => month,
        _ => current.next(),
    }
}

/// What filling `target` from `source` would write: a Month-only amount for each leaf budgeted in
/// the month before, skipping any with its own record in `target` and any the source leaves
/// where the plan already has it.
pub fn fill_preview(
    budget: &Budget,
    ledger: &Ledger<'_>,
    target: Period,
    source: FillSource,
) -> FillPreview {
    let index = SpentIndex::build(budget, ledger);
    let previous = target.prev();
    let mut preview = FillPreview {
        target,
        source,
        changes: Vec::new(),
        kept: Vec::new(),
        unchanged: 0,
        total: Money(zero()),
    };
    for id in expense_leaves(ledger.categories) {
        let chain = budget.chain(id);
        let planned = applied(chain, target).map(|found| found.amount);
        let filled = applied(chain, previous).and_then(|last| {
            if chain.iter().any(|record| record.month == target) {
                preview.kept.push(id);
                return None;
            }
            let after = match source {
                FillSource::PreviousMonth => last.amount,
                FillSource::Average => average_of(&index, id, target),
            };
            if planned.as_ref() == Some(&after) {
                preview.unchanged += 1;
                return None;
            }
            Some(after)
        });
        let ends_at = filled.clone().or_else(|| planned.clone());
        if let Some(amount) = ends_at {
            preview.total.0 += amount.0;
        }
        if let Some(after) = filled {
            preview.changes.push(FillLine {
                category_id: id,
                before: planned,
                after,
            });
        }
    }
    preview
}

/// The average signed Income Splits on the Budget's Accounts over the last 3 closed months.
pub fn average_income(budget: &Budget, ledger: &Ledger<'_>, today: NaiveDate) -> Money {
    let last_three: Vec<Period> = months_before(Period::of(today)).collect();
    let income: Vec<u32> = ledger
        .categories
        .iter()
        .filter(|c| c.category_type == CategoryTypes::Income)
        .map(|c| c.id)
        .collect();
    let mut total = zero();
    for transaction in ledger
        .transactions
        .iter()
        .filter(|t| budget.account_ids.contains(&t.account_id))
        .filter(|t| last_three.contains(&Period::of(t.date)))
    {
        for split in transaction
            .splits
            .iter()
            .filter(|s| income.contains(&s.category_id))
        {
            total += split.amount.0.clone();
        }
    }
    Money((total / BigDecimal::from(AVERAGE_MONTHS)).with_scale_round(2, RoundingMode::HalfUp))
}

/// The Plan grid's footer for `month`: average income minus Total budgeted. Negative means
/// over-allocated.
pub fn unallocated(budget: &Budget, ledger: &Ledger<'_>, month: Period, today: NaiveDate) -> Money {
    Money(
        average_income(budget, ledger, today).0
            - total_budgeted(budget, ledger.categories, month).0,
    )
}

// ---------------------------------------------------------------------------------------------
// The Plan grid (9b)
// ---------------------------------------------------------------------------------------------

/// The months the Plan grid shows at once.
pub const PLAN_MONTHS: usize = 6;

/// The cursor column past the last month: the ROLLOVER cell.
pub const PLAN_ROLLOVER_COLUMN: usize = PLAN_MONTHS;

/// How far ahead of the current month the Plan range can start.
const PLAN_MAX_LEAD: i32 = 12;

/// One month of a Category's row.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanCell {
    pub month: Period,
    /// `None` is Unbudgeted (never budgeted, or Stopped).
    pub amount: Option<Money>,
    /// A record starts in this month: shown bold.
    pub own_record: bool,
    /// Before the current month: read-only.
    pub closed: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlanRow {
    pub category_id: u32,
    pub name: String,
    pub cells: Vec<PlanCell>,
    /// The current month's Rollover, `None` while that month is Unbudgeted.
    pub rollover: Option<Rollover>,
}

/// Rows under their Category's direct parent, or under no label for a top-level leaf.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanSection {
    pub parent: Option<String>,
    pub rows: Vec<PlanRow>,
}

/// Everything the Plan tab draws for one range.
#[derive(Debug, Clone, PartialEq)]
pub struct Plan {
    pub months: Vec<Period>,
    pub sections: Vec<PlanSection>,
    /// Total budgeted per month: Budget Amounts only, no carry.
    pub totals: Vec<Money>,
    /// Average income minus each month's total; negative is over-allocated.
    pub unallocated: Vec<Money>,
    pub average_income: Money,
}

impl Plan {
    pub fn row_count(&self) -> usize {
        self.sections.iter().map(|section| section.rows.len()).sum()
    }

    /// The row at a position counted across sections.
    pub fn row(&self, index: usize) -> Option<&PlanRow> {
        self.sections
            .iter()
            .flat_map(|section| section.rows.iter())
            .nth(index)
    }
}

/// The range the Plan opens on: two months of history, the current month, then three ahead.
pub fn default_plan_start(today: NaiveDate) -> Period {
    Period::of(today).prev().prev()
}

/// The earliest and latest range start: the Budget's first Budget Amount (else the default start)
/// to a year ahead of the current month.
pub fn plan_start_bounds(budget: &Budget, today: NaiveDate) -> (Period, Period) {
    let earliest = budget.first_amount_month().map_or_else(
        || default_plan_start(today),
        |first| first.min(default_plan_start(today)),
    );
    let latest = (0..PLAN_MAX_LEAD).fold(default_plan_start(today), |month, _| month.next());
    (earliest, latest)
}

/// The Plan grid's figures for the six months from `start`. A row is any leaf with an amount or
/// a record in those months, so a cell just cleared keeps its row.
pub fn plan(budget: &Budget, ledger: &Ledger<'_>, start: Period, today: NaiveDate) -> Plan {
    let current = Period::of(today);
    let months: Vec<Period> = std::iter::successors(Some(start), |month| Some(month.next()))
        .take(PLAN_MONTHS)
        .collect();
    let mut sections: Vec<PlanSection> = Vec::new();
    for (id, _) in categories::paths_in_tree_order(ledger.categories) {
        if !expense_leaves(ledger.categories).contains(&id) {
            continue;
        }
        let chain = budget.chain(id);
        let cells: Vec<PlanCell> = months
            .iter()
            .map(|month| {
                let found = applied(chain, *month);
                PlanCell {
                    month: *month,
                    own_record: found.as_ref().is_some_and(|f| f.own_record),
                    amount: found.map(|f| f.amount),
                    closed: *month < current,
                }
            })
            .collect();
        let shown = months.iter().zip(&cells).any(|(month, cell)| {
            cell.amount.is_some() || chain.iter().any(|record| record.month == *month)
        });
        let Some(category) = ledger.categories.iter().find(|c| c.id == id) else {
            continue;
        };
        if !shown {
            continue;
        }
        let parent = category
            .parent
            .and_then(|parent| ledger.categories.iter().find(|c| c.id == parent))
            .map(|parent| parent.name.clone());
        let row = PlanRow {
            category_id: id,
            name: category.name.clone(),
            cells,
            rollover: applied(chain, current).map(|found| found.rollover),
        };
        match sections.iter_mut().find(|section| section.parent == parent) {
            Some(section) => section.rows.push(row),
            None => sections.push(PlanSection {
                parent,
                rows: vec![row],
            }),
        }
    }
    // Top-level leaves have no label, so they close the grid as its "Other" group.
    sections.sort_by_key(|section| section.parent.is_none());
    let average_income = average_income(budget, ledger, today);
    let totals: Vec<Money> = months
        .iter()
        .map(|month| total_budgeted(budget, ledger.categories, *month))
        .collect();
    let unallocated = totals
        .iter()
        .map(|total| Money(average_income.0.clone() - total.0.clone()))
        .collect();
    Plan {
        months,
        sections,
        totals,
        unallocated,
        average_income,
    }
}

/// The cell being typed into. The first key replaces the prefilled amount, as in a spreadsheet;
/// after that keys edit it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanEdit {
    pub category_id: u32,
    pub month: Period,
    pub text: String,
    fresh: bool,
}

impl PlanEdit {
    pub fn new(category_id: u32, month: Period, current: Option<&Money>) -> Self {
        Self {
            category_id,
            month,
            text: current.map(|money| money.0.to_string()).unwrap_or_default(),
            fresh: true,
        }
    }

    /// Digits and one decimal point; anything else is ignored.
    pub fn type_char(&mut self, c: char) {
        if !(c.is_ascii_digit() || c == '.') {
            return;
        }
        if self.fresh {
            self.text.clear();
            self.fresh = false;
        }
        if c == '.' && self.text.contains('.') {
            return;
        }
        self.text.push(c);
    }

    pub fn backspace(&mut self) {
        if self.fresh {
            self.text.clear();
            self.fresh = false;
        } else {
            self.text.pop();
        }
    }
}

// ---------------------------------------------------------------------------------------------
// The store and its operations
// ---------------------------------------------------------------------------------------------

/// Errors for Budget and Category Limit operations.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BudgetError {
    #[error("budget not found")]
    NotFound,
    #[error("a budget needs a name")]
    NameRequired,
    #[error("another budget already has that name")]
    NameTaken,
    #[error("an archived budget is read-only")]
    Archived,
    #[error("the default budget can't be archived")]
    DefaultCannotBeArchived,
    #[error("only a leaf expense category can be budgeted")]
    NotAnExpenseLeaf,
    #[error("closed months are read-only")]
    ClosedMonth,
    #[error("a budget amount can't be negative")]
    NegativeAmount,
    #[error("account not found")]
    AccountNotFound,
    #[error("an on-budget account must take transactions in the budget's unit")]
    AccountNotInUnit,
    #[error("that category has no budget amount this month")]
    NotBudgeted,
    #[error("that isn't an amount")]
    InvalidAmount,
}

/// Where a new Budget's Category Limits come from (11c's Start from).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartFrom {
    Empty,
    /// The source Budget's current-month amounts plus Rollover.
    Copy(u32),
    /// The 3-month average on the chosen Accounts; a 0 average leaves the Category Unbudgeted.
    LastThreeMonths,
}

/// What 11c submits.
#[derive(Debug, Clone, PartialEq)]
pub struct NewBudget {
    pub name: String,
    pub unit: String,
    pub account_ids: Vec<u32>,
    pub start_from: StartFrom,
}

/// Whether a Budget Amount runs onward or for its month alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Span {
    Onward,
    MonthOnly,
}

/// Every Budget, in creation order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Budgets {
    list: Vec<Budget>,
}

impl Budgets {
    pub fn all(&self) -> &[Budget] {
        &self.list
    }

    pub fn get(&self, id: u32) -> Option<&Budget> {
        self.list.iter().find(|budget| budget.id == id)
    }

    pub fn active(&self) -> impl Iterator<Item = &Budget> {
        self.list.iter().filter(|budget| !budget.is_archived())
    }

    pub fn archived(&self) -> impl Iterator<Item = &Budget> {
        self.list.iter().filter(|budget| budget.is_archived())
    }

    /// The Budget the rail badge, the Dashboard and first launch read.
    pub fn default_budget(&self) -> Option<&Budget> {
        self.list.iter().find(|budget| budget.is_default)
    }

    fn get_mut(&mut self, id: u32) -> Result<&mut Budget, BudgetError> {
        self.list
            .iter_mut()
            .find(|budget| budget.id == id)
            .ok_or(BudgetError::NotFound)
    }

    fn editable_mut(&mut self, id: u32) -> Result<&mut Budget, BudgetError> {
        let budget = self.get_mut(id)?;
        if budget.is_archived() {
            return Err(BudgetError::Archived);
        }
        Ok(budget)
    }

    fn next_id(&self) -> u32 {
        self.list.iter().map(|b| b.id).max().unwrap_or(0) + 1
    }

    fn name_taken(&self, name: &str, except: Option<u32>) -> bool {
        self.list
            .iter()
            .any(|b| Some(b.id) != except && b.name.eq_ignore_ascii_case(name))
    }

    /// Creates a Budget (never the default unless it is the first) and returns its id.
    pub fn create(
        &mut self,
        new: &NewBudget,
        ledger: &Ledger<'_>,
        today: NaiveDate,
    ) -> Result<u32, BudgetError> {
        let name = new.name.trim();
        if name.is_empty() {
            return Err(BudgetError::NameRequired);
        }
        if self.name_taken(name, None) {
            return Err(BudgetError::NameTaken);
        }
        check_accounts(ledger.accounts, &new.unit, &new.account_ids)?;
        let copy_from = match &new.start_from {
            StartFrom::Copy(source) => {
                Some(self.get(*source).ok_or(BudgetError::NotFound)?.clone())
            }
            StartFrom::Empty | StartFrom::LastThreeMonths => None,
        };
        let current = Period::of(today);
        let mut budget = Budget {
            id: self.next_id(),
            name: name.to_string(),
            method: Method::Limits,
            unit: new.unit.clone(),
            account_ids: new.account_ids.clone(),
            is_default: self.list.is_empty(),
            archived_at: None,
            limits: BTreeMap::new(),
        };
        if let Some(source) = copy_from {
            for id in expense_leaves(ledger.categories) {
                if let Some(found) = applied(source.chain(id), current) {
                    put(
                        budget.limits.entry(id).or_default(),
                        LimitRecord {
                            month: current,
                            limit: Limit::Onward {
                                amount: found.amount,
                                rollover: found.rollover,
                            },
                        },
                    );
                }
            }
        } else if new.start_from == StartFrom::LastThreeMonths {
            for (id, average) in three_month_averages(&budget, ledger, current) {
                if !average.0.is_zero() {
                    put(
                        budget.limits.entry(id).or_default(),
                        LimitRecord {
                            month: current,
                            limit: Limit::Onward {
                                amount: average,
                                rollover: Rollover::None,
                            },
                        },
                    );
                }
            }
        }
        let id = budget.id;
        self.list.push(budget);
        Ok(id)
    }

    /// 11c's edit mode: the one place to rename a Budget or change its Accounts (Unit and Method
    /// stay locked).
    pub fn edit(
        &mut self,
        id: u32,
        name: &str,
        account_ids: Vec<u32>,
        accounts: &[Account],
    ) -> Result<(), BudgetError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(BudgetError::NameRequired);
        }
        if self.name_taken(name, Some(id)) {
            return Err(BudgetError::NameTaken);
        }
        let budget = self.editable_mut(id)?;
        check_accounts(accounts, &budget.unit, &account_ids)?;
        budget.name = name.to_string();
        budget.account_ids = account_ids;
        Ok(())
    }

    /// Copies Unit, Method, Accounts and the full chains as "<name> copy" (numbered when taken).
    /// The copy is never the default and is never archived.
    pub fn duplicate(&mut self, id: u32) -> Result<u32, BudgetError> {
        let source = self.get(id).ok_or(BudgetError::NotFound)?.clone();
        let mut name = format!("{} copy", source.name);
        let mut number = 2;
        while self.name_taken(&name, None) {
            name = format!("{} copy {number}", source.name);
            number += 1;
        }
        let copy = Budget {
            id: self.next_id(),
            name,
            is_default: false,
            archived_at: None,
            ..source
        };
        let new_id = copy.id;
        self.list.push(copy);
        Ok(new_id)
    }

    /// Archiving never deletes, and never the default.
    pub fn archive(&mut self, id: u32, today: NaiveDate) -> Result<(), BudgetError> {
        let budget = self.get_mut(id)?;
        if budget.is_default {
            return Err(BudgetError::DefaultCannotBeArchived);
        }
        budget.archived_at.get_or_insert(today);
        Ok(())
    }

    pub fn restore(&mut self, id: u32) -> Result<(), BudgetError> {
        self.get_mut(id)?.archived_at = None;
        Ok(())
    }

    /// Makes an active Budget the one default.
    pub fn set_default(&mut self, id: u32) -> Result<(), BudgetError> {
        self.editable_mut(id)?;
        for budget in &mut self.list {
            budget.is_default = budget.id == id;
        }
        Ok(())
    }

    /// Writes an Onward or Month-only Budget Amount starting in `month`, replacing whatever record
    /// starts there. `rollover` of `None` inherits the record it follows.
    #[expect(
        clippy::too_many_arguments,
        reason = "one call per Plan-grid or Edit budget save"
    )]
    pub fn set_amount(
        &mut self,
        id: u32,
        categories: &[Category],
        category_id: u32,
        month: Period,
        span: Span,
        amount: Money,
        rollover: Option<Rollover>,
        today: NaiveDate,
    ) -> Result<(), BudgetError> {
        check_leaf(categories, category_id)?;
        check_open_month(month, today)?;
        if amount.0.is_negative() {
            return Err(BudgetError::NegativeAmount);
        }
        let budget = self.editable_mut(id)?;
        let chain = budget.limits.entry(category_id).or_default();
        let rollover = rollover.unwrap_or_else(|| inherited_rollover(chain, month));
        let limit = match span {
            Span::Onward => Limit::Onward { amount, rollover },
            Span::MonthOnly => Limit::MonthOnly { amount, rollover },
        };
        put(chain, LimitRecord { month, limit });
        Ok(())
    }

    /// Writes a Stop from `month`. A Category with no amount there and no record starting there
    /// has nothing to stop, so nothing is written.
    pub fn stop(
        &mut self,
        id: u32,
        categories: &[Category],
        category_id: u32,
        month: Period,
        today: NaiveDate,
    ) -> Result<(), BudgetError> {
        check_leaf(categories, category_id)?;
        check_open_month(month, today)?;
        let budget = self.editable_mut(id)?;
        let chain = budget.limits.entry(category_id).or_default();
        let has_record = chain.iter().any(|record| record.month == month);
        if has_record || applied(chain, month).is_some() {
            put(
                chain,
                LimitRecord {
                    month,
                    limit: Limit::Stop,
                },
            );
        }
        if chain.is_empty() {
            budget.limits.remove(&category_id);
        }
        Ok(())
    }

    /// Removes the Month-only record starting in `month`, so the month falls back to the carried
    /// Onward value. Returns whether one was removed.
    pub fn clear_month_only(
        &mut self,
        id: u32,
        category_id: u32,
        month: Period,
        today: NaiveDate,
    ) -> Result<bool, BudgetError> {
        check_open_month(month, today)?;
        let budget = self.editable_mut(id)?;
        let Some(chain) = budget.limits.get_mut(&category_id) else {
            return Ok(false);
        };
        let before = chain.len();
        chain.retain(|r| !(r.month == month && matches!(r.limit, Limit::MonthOnly { .. })));
        let removed = chain.len() != before;
        if chain.is_empty() {
            budget.limits.remove(&category_id);
        }
        Ok(removed)
    }

    /// The Plan grid's `r`: cycles the current month's Rollover, writing an Onward record from the
    /// current month with the same amount.
    pub fn cycle_rollover(
        &mut self,
        id: u32,
        category_id: u32,
        today: NaiveDate,
    ) -> Result<Rollover, BudgetError> {
        let current = Period::of(today);
        let budget = self.editable_mut(id)?;
        let chain = budget.limits.entry(category_id).or_default();
        let Some(found) = applied(chain, current) else {
            return Err(BudgetError::NotBudgeted);
        };
        let next = found.rollover.next();
        put(
            chain,
            LimitRecord {
                month: current,
                limit: Limit::Onward {
                    amount: found.amount,
                    rollover: next,
                },
            },
        );
        Ok(next)
    }

    /// Saves what was typed into a Plan cell. An amount writes an Onward or Month-only record
    /// (`0` is a real 0.00). Empty text clears: Onward writes a Stop from `month`, Month-only
    /// removes that month's own Month-only record. Returns whether anything changed.
    #[expect(
        clippy::too_many_arguments,
        reason = "one call per Plan-grid cell save"
    )]
    pub fn save_cell(
        &mut self,
        id: u32,
        categories: &[Category],
        category_id: u32,
        month: Period,
        span: Span,
        text: &str,
        today: NaiveDate,
    ) -> Result<bool, BudgetError> {
        let text = text.trim();
        if text.is_empty() {
            return match span {
                Span::Onward => {
                    let before = self.get(id).ok_or(BudgetError::NotFound)?.clone();
                    self.stop(id, categories, category_id, month, today)?;
                    Ok(self.get(id) != Some(&before))
                }
                Span::MonthOnly => self.clear_month_only(id, category_id, month, today),
            };
        }
        let amount: Money = text.parse().map_err(|_| BudgetError::InvalidAmount)?;
        let amount = Money(amount.0.with_scale_round(2, RoundingMode::HalfUp));
        let chain = self
            .get(id)
            .ok_or(BudgetError::NotFound)?
            .chain(category_id);
        let unchanged = applied(chain, month).is_some_and(|found| found.amount == amount)
            && !chain
                .iter()
                .any(|r| r.month == month && matches!(r.limit, Limit::MonthOnly { .. }));
        if span == Span::Onward && unchanged {
            return Ok(false);
        }
        self.set_amount(
            id,
            categories,
            category_id,
            month,
            span,
            amount,
            None,
            today,
        )?;
        Ok(true)
    }

    /// Fill (9f): writes `source`'s Month-only amounts into `target`, as [`fill_preview`] lists
    /// them. Returns how many Categories it wrote.
    pub fn fill(
        &mut self,
        id: u32,
        ledger: &Ledger<'_>,
        target: Period,
        source: FillSource,
        today: NaiveDate,
    ) -> Result<usize, BudgetError> {
        check_open_month(target, today)?;
        let budget = self.editable_mut(id)?;
        let preview = fill_preview(budget, ledger, target, source);
        for line in &preview.changes {
            let chain = budget.limits.entry(line.category_id).or_default();
            let rollover = inherited_rollover(chain, target);
            put(
                chain,
                LimitRecord {
                    month: target,
                    limit: Limit::MonthOnly {
                        amount: line.after.clone(),
                        rollover,
                    },
                },
            );
        }
        Ok(preview.changes.len())
    }

    /// Categories 5c's write: a changed value is an Onward from the current month, clearing is a
    /// Stop from it, and an unchanged value writes nothing. Returns whether it wrote.
    pub fn set_monthly_limit(
        &mut self,
        id: u32,
        categories: &[Category],
        category_id: u32,
        amount: Option<Money>,
        today: NaiveDate,
    ) -> Result<bool, BudgetError> {
        let current = Period::of(today);
        let existing = self
            .get(id)
            .ok_or(BudgetError::NotFound)?
            .chain(category_id);
        let existing = applied(existing, current).map(|found| found.amount);
        if existing == amount {
            return Ok(false);
        }
        match amount {
            Some(amount) => self.set_amount(
                id,
                categories,
                category_id,
                current,
                Span::Onward,
                amount,
                None,
                today,
            )?,
            None => self.stop(id, categories, category_id, current, today)?,
        }
        Ok(true)
    }

    /// The current month's Budget Amount as 5c shows it (Month-only included). A parent is a
    /// read-only rollup: the sum of its budgeted leaves, `None` when none is.
    pub fn monthly_limit(
        &self,
        id: u32,
        categories: &[Category],
        category_id: u32,
        today: NaiveDate,
    ) -> Option<Money> {
        let budget = self.get(id)?;
        let current = Period::of(today);
        let amounts: Vec<BigDecimal> = categories::descendants_inclusive(categories, category_id)
            .into_iter()
            .filter(|below| categories::is_leaf(categories, *below))
            .filter_map(|below| applied(budget.chain(below), current))
            .map(|found| found.amount.0)
            .collect();
        (!amounts.is_empty()).then(|| Money(amounts.into_iter().fold(zero(), |a, b| a + b)))
    }

    /// How many Budgets hold a chain for the Category: the Delete Category dialog's count.
    pub fn limit_count(&self, category_id: u32) -> usize {
        self.list
            .iter()
            .filter(|budget| budget.limits.contains_key(&category_id))
            .count()
    }

    /// A deleted Category's records go with it, in every Budget (archived ones too).
    pub fn remove_category(&mut self, category_id: u32) {
        for budget in &mut self.list {
            budget.limits.remove(&category_id);
        }
    }

    /// A budgeted leaf that gained a child gets an automatic Stop from the current month and loses
    /// any later record, in every active Budget; a parent only ever rolls up. Returns the Categories
    /// stopped.
    pub fn stop_new_parents(&mut self, categories: &[Category], today: NaiveDate) -> Vec<u32> {
        let current = Period::of(today);
        let mut stopped = Vec::new();
        for budget in self.list.iter_mut().filter(|b| !b.is_archived()) {
            for (id, chain) in &mut budget.limits {
                if categories::is_leaf(categories, *id) {
                    continue;
                }
                let was_budgeted = applied(chain, current).is_some();
                chain.retain(|record| record.month < current);
                if was_budgeted {
                    put(
                        chain,
                        LimitRecord {
                            month: current,
                            limit: Limit::Stop,
                        },
                    );
                    if !stopped.contains(id) {
                        stopped.push(*id);
                    }
                }
            }
        }
        stopped
    }
}

fn check_leaf(categories: &[Category], category_id: u32) -> Result<(), BudgetError> {
    let expense = categories
        .iter()
        .any(|c| c.id == category_id && c.category_type == CategoryTypes::Expense);
    if expense && categories::is_leaf(categories, category_id) {
        Ok(())
    } else {
        Err(BudgetError::NotAnExpenseLeaf)
    }
}

fn check_open_month(month: Period, today: NaiveDate) -> Result<(), BudgetError> {
    if month < Period::of(today) {
        Err(BudgetError::ClosedMonth)
    } else {
        Ok(())
    }
}

fn check_accounts(accounts: &[Account], unit: &str, ids: &[u32]) -> Result<(), BudgetError> {
    for id in ids {
        let account = accounts
            .iter()
            .find(|a| a.id == *id)
            .ok_or(BudgetError::AccountNotFound)?;
        if !transactions::takes_transactions(&account.account_type) || account.unit != unit {
            return Err(BudgetError::AccountNotInUnit);
        }
    }
    Ok(())
}

/// Whether spending (an expense amount, negative) exceeds the budget for this category.
/// Categories 5c's row bar reads it against the month-to-date sum, which keeps the ledger's sign.
pub fn is_over_budget(spent: &Money, budget: &Money) -> bool {
    spent.0 < budget.0.clone() * -1 // spent is negative; over-budget when |spent| > budget
}

// ---------------------------------------------------------------------------------------------
// Surface state
// ---------------------------------------------------------------------------------------------

/// Which tab of the Budgets surface shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BudgetsTab {
    #[default]
    Progress,
    Plan,
    History,
}

impl BudgetsTab {
    /// `tab` cycles forward, wrapping.
    pub fn next(self) -> Self {
        match self {
            BudgetsTab::Progress => BudgetsTab::Plan,
            BudgetsTab::Plan => BudgetsTab::History,
            BudgetsTab::History => BudgetsTab::Progress,
        }
    }

    /// `1`/`2`/`3` pick a tab directly.
    pub fn from_digit(digit: char) -> Option<Self> {
        match digit {
            '1' => Some(BudgetsTab::Progress),
            '2' => Some(BudgetsTab::Plan),
            '3' => Some(BudgetsTab::History),
            _ => None,
        }
    }
}

/// What confirming the Switcher does: open the highlighted Budget, or start a new one (`n`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SwitcherRequest {
    #[default]
    Choose,
    New,
}

/// The Switcher popover's state (11b). The list takes `j`/`k`/`n` until `/` hands the keys to the
/// search field. The Budgets it lists are copied in as it opens, so its keys never need the
/// ledger.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Switcher {
    pub query: TextField,
    pub searching: bool,
    /// Index into [`Self::ids`] for the query.
    pub selected: usize,
    /// Every Budget's id and name: active ones in creation order, then archived ones.
    entries: Vec<(u32, String)>,
    pub request: SwitcherRequest,
}

impl Switcher {
    /// Opens with `current` highlighted.
    pub fn new(budgets: &Budgets, current: u32) -> Self {
        let entries: Vec<(u32, String)> = budgets
            .active()
            .chain(budgets.archived())
            .map(|budget| (budget.id, budget.name.clone()))
            .collect();
        Self {
            selected: entries
                .iter()
                .position(|(id, _)| *id == current)
                .unwrap_or(0),
            entries,
            ..Self::default()
        }
    }

    /// The rows for the query, as [`switcher_ids`] gives them.
    pub fn ids(&self) -> Vec<u32> {
        let query = self.query.text().trim().to_lowercase();
        self.entries
            .iter()
            .filter(|(_, name)| query.is_empty() || name.to_lowercase().contains(&query))
            .map(|(id, _)| *id)
            .collect()
    }

    /// Moves the highlight, stopping at either end of the matching rows.
    pub fn step(&mut self, forward: bool) {
        let last = self.ids().len().saturating_sub(1);
        self.selected = if forward {
            (self.selected + 1).min(last)
        } else {
            self.selected.min(last).saturating_sub(1)
        };
    }

    /// The highlighted Budget, if any row matches.
    pub fn chosen(&self) -> Option<u32> {
        let ids = self.ids();
        ids.get(self.selected.min(ids.len().saturating_sub(1)))
            .copied()
    }

    pub fn type_char(&mut self, c: char) {
        if !c.is_control() {
            self.query.push(c);
            self.selected = 0;
        }
    }

    pub fn backspace(&mut self) {
        self.query.backspace();
        self.selected = 0;
    }
}

impl Dialog for Switcher {
    /// With the list focused `j`/`k` move, `n` starts a new Budget and `/` hands the keys to the
    /// search field; while searching, every character narrows the list and only the arrows move.
    /// `Enter` opens the highlighted row.
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        let searching = self.searching;
        match key {
            DialogKey::Down => self.step(true),
            DialogKey::Up => self.step(false),
            DialogKey::Backspace if searching => self.backspace(),
            DialogKey::Char('j') if !searching => self.step(true),
            DialogKey::Char('k') if !searching => self.step(false),
            DialogKey::Char('/') if !searching => self.searching = true,
            DialogKey::Char('n') if !searching => {
                self.request = SwitcherRequest::New;
                return Some(DialogOutcome::Confirm);
            }
            DialogKey::Char(ch) if searching => self.type_char(ch),
            _ => return None,
        }
        Some(DialogOutcome::Handled)
    }

    fn is_valid(&self) -> bool {
        self.request == SwitcherRequest::New || self.chosen().is_some()
    }

    /// The first `Esc` gives the keys back to the list before the popover closes.
    fn close_open_select(&mut self) -> bool {
        std::mem::take(&mut self.searching)
    }
}

/// The Switcher's rows: active Budgets in creation order, then archived ones, keeping those whose
/// name contains `query` whatever its case.
pub fn switcher_ids(budgets: &Budgets, query: &str) -> Vec<u32> {
    let query = query.trim().to_lowercase();
    budgets
        .active()
        .chain(budgets.archived())
        .filter(|budget| query.is_empty() || budget.name.to_lowercase().contains(&query))
        .map(|budget| budget.id)
        .collect()
}

/// What a Manage budgets row's action does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManageAction {
    Open,
    Edit,
    Duplicate,
    SetDefault,
    Archive,
    Restore,
}

/// What a key in Manage budgets asks `Shell` to do once it confirms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManageRequest {
    New,
    Action(u32, ManageAction),
}

/// 11f: the row cursor over the Switcher's rows, and the last refused action. The rows' ids and
/// whether each is archived are copied in as it opens and refreshed after an action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manage {
    pub selected: usize,
    pub error: Option<BudgetError>,
    rows: Vec<(u32, bool)>,
    pub request: Option<ManageRequest>,
}

impl Manage {
    /// Opens with Budget `current` under the cursor.
    pub fn new(budgets: &Budgets, current: u32) -> Self {
        let mut manage = Self {
            selected: 0,
            error: None,
            rows: Vec::new(),
            request: None,
        };
        manage.refresh(budgets, current);
        manage
    }

    /// Rebuilds the rows and follows `id` to where it now sits. Archiving or restoring moves a
    /// row between the two sections.
    pub fn refresh(&mut self, budgets: &Budgets, id: u32) {
        self.rows = budgets
            .active()
            .map(|budget| (budget.id, false))
            .chain(budgets.archived().map(|budget| (budget.id, true)))
            .collect();
        self.selected = self
            .rows
            .iter()
            .position(|(each, _)| *each == id)
            .unwrap_or(0);
    }
}

impl Dialog for Manage {
    /// `j`/`k` move, `enter` opens, `n` starts a new Budget, `e` edits, `d` duplicates, `*` sets
    /// the default and `x` archives or restores. Each but the moves is a request `Shell` runs.
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        let last = self.rows.len().checked_sub(1)?;
        let at = self.selected.min(last);
        let (id, archived) = self.rows[at];
        let action = match key {
            DialogKey::Down | DialogKey::Char('j') => {
                self.selected = (at + 1).min(last);
                return Some(DialogOutcome::Handled);
            }
            DialogKey::Up | DialogKey::Char('k') => {
                self.selected = at.saturating_sub(1);
                return Some(DialogOutcome::Handled);
            }
            DialogKey::Char('n') => {
                self.request = Some(ManageRequest::New);
                return Some(DialogOutcome::Confirm);
            }
            DialogKey::Enter => ManageAction::Open,
            DialogKey::Char('e') => ManageAction::Edit,
            DialogKey::Char('d') => ManageAction::Duplicate,
            DialogKey::Char('*') => ManageAction::SetDefault,
            DialogKey::Char('x') if archived => ManageAction::Restore,
            DialogKey::Char('x') => ManageAction::Archive,
            _ => return None,
        };
        self.request = Some(ManageRequest::Action(id, action));
        Some(DialogOutcome::Confirm)
    }

    fn is_valid(&self) -> bool {
        self.request.is_some()
    }
}

/// What a key on 9d asks `Shell` to do once it confirms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailRequest {
    /// `t` or `enter`: hand off to Transactions.
    Transactions,
    /// `e`: Edit budget.
    Edit,
}

/// 9d for a Category (a leaf or a parent's rollup) and month: the Transaction row cursor, over
/// `listed` rows counted as it opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detail {
    pub category_id: u32,
    pub month: Period,
    pub selected: usize,
    pub listed: usize,
    pub request: Option<DetailRequest>,
}

impl Detail {
    pub fn new(category_id: u32, month: Period, listed: usize) -> Self {
        Self {
            category_id,
            month,
            selected: 0,
            listed,
            request: None,
        }
    }
}

impl Dialog for Detail {
    /// `j`/`k` move the cursor, `t` or `enter` hand off to Transactions and `e` opens Edit budget.
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        match key {
            DialogKey::Down | DialogKey::Char('j') => {
                self.selected = accounts::step_selection(self.selected, self.listed, 1);
            }
            DialogKey::Up | DialogKey::Char('k') => {
                self.selected = accounts::step_selection(self.selected, self.listed, -1);
            }
            DialogKey::Enter | DialogKey::Char('t') => {
                self.request = Some(DetailRequest::Transactions);
                return Some(DialogOutcome::Confirm);
            }
            DialogKey::Char('e') => {
                self.request = Some(DetailRequest::Edit);
                return Some(DialogOutcome::Confirm);
            }
            _ => return None,
        }
        Some(DialogOutcome::Handled)
    }

    fn is_valid(&self) -> bool {
        self.request.is_some()
    }
}

/// The open Budgets dialog. Each screen's ticket adds its form to its variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetsDialog {
    /// 11b, the popover under the title.
    Switcher(Switcher),
    /// 11c creating a Budget, or its edit mode (the form names the Budget).
    Budget(BudgetForm),
    /// 11f.
    Manage(Manage),
    /// 9d.
    CategoryDetail(Detail),
    /// 9e: editing a Category's amount, or picking an unbudgeted Category to budget.
    EditLimit(LimitForm),
    /// 9f, filling this month from the chosen source.
    Fill { month: Period, source: FillSource },
    /// 9g for a Category.
    Stop(StopForm),
}

impl BudgetsDialog {
    fn inner(&self) -> Option<&dyn Dialog> {
        match self {
            Self::Switcher(dialog) => Some(dialog),
            Self::Budget(dialog) => Some(dialog),
            Self::Manage(dialog) => Some(dialog),
            Self::CategoryDetail(dialog) => Some(dialog),
            Self::EditLimit(dialog) => Some(dialog),
            Self::Stop(dialog) => Some(dialog),
            Self::Fill { .. } => None,
        }
    }

    fn inner_mut(&mut self) -> Option<&mut dyn Dialog> {
        match self {
            Self::Switcher(dialog) => Some(dialog),
            Self::Budget(dialog) => Some(dialog),
            Self::Manage(dialog) => Some(dialog),
            Self::CategoryDetail(dialog) => Some(dialog),
            Self::EditLimit(dialog) => Some(dialog),
            Self::Stop(dialog) => Some(dialog),
            Self::Fill { .. } => None,
        }
    }
}

impl Dialog for BudgetsDialog {
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        if let Self::Fill { source, .. } = self {
            // `j`/`k` pick the source; `enter` fills.
            let all = FillSource::ALL;
            let at = all.iter().position(|each| each == source).unwrap_or(0);
            match key {
                DialogKey::Down | DialogKey::Char('j') => {
                    *source = all[(at + 1).min(all.len() - 1)]
                }
                DialogKey::Up | DialogKey::Char('k') => *source = all[at.saturating_sub(1)],
                _ => return None,
            }
            return Some(DialogOutcome::Handled);
        }
        self.inner_mut()?.handle_own_key(key)
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        self.inner_mut()?.focused_text()
    }

    fn cycle_field(&mut self) {
        if let Some(dialog) = self.inner_mut() {
            dialog.cycle_field();
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

/// One seeded Category Limit: Onward from `from` months before the current month, with an
/// optional later change.
struct LimitSeed {
    category: &'static str,
    cents: i64,
    rollover: Rollover,
    /// Months relative to the current month that the amount starts in.
    from: i32,
    /// A later Onward `(months from now, cents)`.
    then: Option<(i32, i64)>,
}

const fn seed(category: &'static str, cents: i64, rollover: Rollover, from: i32) -> LimitSeed {
    LimitSeed {
        category,
        cents,
        rollover,
        from,
        then: None,
    }
}

const PERSONAL_SPENDING: [LimitSeed; 7] = [
    seed("Rent", 100_000, Rollover::None, -6),
    seed("Electricity", 9_000, Rollover::CarryUnspent, -6),
    seed("Water", 6_000, Rollover::None, -6),
    seed("Groceries", 50_000, Rollover::None, -6),
    LimitSeed {
        then: Some((1, 30_000)),
        ..seed("Dining", 25_000, Rollover::None, -6)
    },
    seed("Transport", 60_000, Rollover::None, -6),
    seed("Household", 40_000, Rollover::None, -3),
];

const JOINT_HOUSEHOLD: [LimitSeed; 3] = [
    seed("Groceries", 70_000, Rollover::None, -4),
    seed("Electricity", 18_000, Rollover::CarryBoth, -4),
    seed("Household", 35_000, Rollover::None, -4),
];

fn month_from(today: NaiveDate, offset: i32) -> Period {
    (0..offset.unsigned_abs()).fold(Period::of(today), |month, _| {
        if offset < 0 {
            month.prev()
        } else {
            month.next()
        }
    })
}

fn seeded_limits(
    seeds: &[LimitSeed],
    categories: &[Category],
    today: NaiveDate,
) -> BTreeMap<u32, Vec<LimitRecord>> {
    let mut limits: BTreeMap<u32, Vec<LimitRecord>> = BTreeMap::new();
    for seed in seeds {
        let Some(id) = categories::find_by_name(categories, seed.category) else {
            continue;
        };
        let chain = limits.entry(id).or_default();
        put(
            chain,
            LimitRecord {
                month: month_from(today, seed.from),
                limit: Limit::Onward {
                    amount: cents_money(seed.cents),
                    rollover: seed.rollover,
                },
            },
        );
        if let Some((offset, cents)) = seed.then {
            put(
                chain,
                LimitRecord {
                    month: month_from(today, offset),
                    limit: Limit::Onward {
                        amount: cents_money(cents),
                        rollover: seed.rollover,
                    },
                },
            );
        }
    }
    limits
}

/// The seeded Budgets: **Personal spending** (migrated, the default, on every AUD Account that
/// takes Transactions) and **Joint household**, a second Limits Budget over two of the same
/// Accounts and some of the same Categories, so the two overlap. A Category or Account the stubs
/// lack is left out.
pub fn default_budgets(accounts: &[Account], categories: &[Category], today: NaiveDate) -> Budgets {
    let on_budget = |names: Option<&[&str]>| -> Vec<u32> {
        accounts
            .iter()
            .filter(|a| a.unit == "aud" && transactions::takes_transactions(&a.account_type))
            .filter(|a| names.is_none_or(|names| names.contains(&a.name.as_str())))
            .map(|a| a.id)
            .collect()
    };
    Budgets {
        list: vec![
            Budget {
                id: PERSONAL_SPENDING_ID,
                name: "Personal spending".to_string(),
                method: Method::Limits,
                unit: "aud".to_string(),
                account_ids: on_budget(None),
                is_default: true,
                archived_at: None,
                limits: seeded_limits(&PERSONAL_SPENDING, categories, today),
            },
            Budget {
                id: 2,
                name: "Joint household".to_string(),
                method: Method::Limits,
                unit: "aud".to_string(),
                account_ids: on_budget(Some(&["ANZ Everyday", "ANZ Offset"])),
                is_default: false,
                archived_at: None,
                limits: seeded_limits(&JOINT_HOUSEHOLD, categories, today),
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        accounts::default_accounts,
        bills::{self, AmountKind, BillPlanDraft, Recurrence},
        categories::default_categories,
        payees::default_payees,
        tags::default_tags,
        transactions::{Split, default_transactions},
    };
    use lib_core::TransactionStatus;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).unwrap_or_default()
    }

    fn today() -> NaiveDate {
        date(2026, 9, 21)
    }

    fn sep() -> Period {
        Period::of(today())
    }

    fn money(text: &str) -> Money {
        text.parse().expect("a valid test amount")
    }

    fn category_id(categories: &[Category], name: &str) -> u32 {
        categories::find_by_name(categories, name).unwrap_or_default()
    }

    /// A tiny hand-built Ledger, so figures are exact rather than random.
    struct World {
        accounts: Vec<Account>,
        categories: Vec<Category>,
        transactions: Vec<Transaction>,
        plans: Vec<BillPlan>,
        entries: Vec<BillScheduleEntry>,
        next_transaction: u32,
    }

    impl World {
        fn new() -> Self {
            Self {
                accounts: default_accounts(),
                categories: default_categories(),
                transactions: Vec::new(),
                plans: Vec::new(),
                entries: Vec::new(),
                next_transaction: 1,
            }
        }

        fn ledger(&self) -> Ledger<'_> {
            Ledger {
                categories: &self.categories,
                accounts: &self.accounts,
                transactions: &self.transactions,
                plans: &self.plans,
                entries: &self.entries,
            }
        }

        fn account(&self, name: &str) -> u32 {
            self.accounts
                .iter()
                .find(|a| a.name == name)
                .map(|a| a.id)
                .unwrap_or_default()
        }

        fn category(&self, name: &str) -> u32 {
            category_id(&self.categories, name)
        }

        /// A single-Split Transaction; `amount` follows the ledger's sign (expenses negative).
        fn post(&mut self, account: &str, category: &str, on: NaiveDate, amount: &str) {
            let id = self.next_transaction;
            self.next_transaction += 1;
            self.transactions.push(Transaction {
                id,
                date: on,
                account_id: self.account(account),
                status: TransactionStatus::Open,
                is_flagged: false,
                description: None,
                splits: vec![Split {
                    amount: money(amount),
                    category_id: self.category(category),
                    payee_id: None,
                    tag_ids: Vec::new(),
                }],
            });
        }

        fn bill(&mut self, account: &str, category: &str, first_due: NaiveDate, amount: &str) {
            let draft = BillPlanDraft {
                name: format!("{category} bill"),
                category_id: self.category(category),
                unit: "aud".to_string(),
                account_id: self.account(account),
                payee_id: None,
                planned_amount: money(amount),
                amount_kind: AmountKind::Fixed,
                recurrence: Recurrence::OneShot,
                first_due,
                ends_on: None,
                attention_lead: None,
            };
            let inserted = bills::insert_plan(
                &mut self.plans,
                &mut self.entries,
                &draft,
                &self.categories,
                &self.accounts,
                today(),
            );
            assert!(inserted.is_ok(), "{inserted:?}");
        }

        /// An empty Budget on the two ANZ accounts.
        fn budgets(&self) -> (Budgets, u32) {
            let mut budgets = Budgets::default();
            let id = budgets
                .create(
                    &NewBudget {
                        name: "Test".to_string(),
                        unit: "aud".to_string(),
                        account_ids: vec![self.account("ANZ Everyday"), self.account("ANZ Offset")],
                        start_from: StartFrom::Empty,
                    },
                    &self.ledger(),
                    today(),
                )
                .unwrap_or_default();
            (budgets, id)
        }
    }

    #[expect(
        clippy::too_many_arguments,
        reason = "a test shorthand for one set_amount call"
    )]
    fn set(
        budgets: &mut Budgets,
        world: &World,
        id: u32,
        category: &str,
        month: Period,
        span: Span,
        amount: &str,
        rollover: Option<Rollover>,
    ) {
        let done = budgets.set_amount(
            id,
            &world.categories,
            world.category(category),
            month,
            span,
            money(amount),
            rollover,
            today(),
        );
        assert_eq!(done, Ok(()));
    }

    fn row(figures: &PeriodFigures, category_id: u32) -> Option<&CategoryFigures> {
        figures.rows.iter().find(|r| r.category_id == category_id)
    }

    // -- chains ---------------------------------------------------------------------------------

    fn onward(month: Period, amount: &str) -> LimitRecord {
        LimitRecord {
            month,
            limit: Limit::Onward {
                amount: money(amount),
                rollover: Rollover::None,
            },
        }
    }

    fn month_only(month: Period, amount: &str) -> LimitRecord {
        LimitRecord {
            month,
            limit: Limit::MonthOnly {
                amount: money(amount),
                rollover: Rollover::None,
            },
        }
    }

    fn stop(month: Period) -> LimitRecord {
        LimitRecord {
            month,
            limit: Limit::Stop,
        }
    }

    fn amount_in(chain: &[LimitRecord], month: Period) -> Option<Money> {
        applied(chain, month).map(|a| a.amount)
    }

    #[test]
    fn an_onward_amount_runs_until_the_next_record() {
        let chain = [
            onward(sep().prev(), "250.00"),
            onward(sep().next(), "300.00"),
        ];
        assert_eq!(amount_in(&chain, sep().prev().prev()), None);
        assert_eq!(amount_in(&chain, sep().prev()), Some(money("250.00")));
        assert_eq!(amount_in(&chain, sep()), Some(money("250.00")));
        assert_eq!(amount_in(&chain, sep().next()), Some(money("300.00")));
        assert_eq!(
            amount_in(&chain, sep().next().next()),
            Some(money("300.00"))
        );
    }

    #[test]
    fn a_month_only_amount_falls_back_to_the_onward_value_the_month_after() {
        let chain = [onward(sep().prev(), "250.00"), month_only(sep(), "400.00")];
        assert_eq!(amount_in(&chain, sep()), Some(money("400.00")));
        assert_eq!(amount_in(&chain, sep().next()), Some(money("250.00")));
    }

    #[test]
    fn a_month_only_amount_with_no_onward_leaves_the_other_months_unbudgeted() {
        let chain = [
            month_only(sep(), "420.00"),
            month_only(sep().next().next().next(), "420.00"),
        ];
        assert_eq!(amount_in(&chain, sep().prev()), None);
        assert_eq!(amount_in(&chain, sep()), Some(money("420.00")));
        assert_eq!(amount_in(&chain, sep().next()), None);
        assert_eq!(
            amount_in(&chain, sep().next().next().next()),
            Some(money("420.00"))
        );
    }

    #[test]
    fn a_stop_ends_the_amount_until_a_later_onward_resumes_it() {
        let chain = [
            onward(sep().prev(), "250.00"),
            stop(sep()),
            onward(sep().next().next(), "275.00"),
        ];
        assert_eq!(amount_in(&chain, sep().prev()), Some(money("250.00")));
        assert_eq!(amount_in(&chain, sep()), None);
        assert_eq!(amount_in(&chain, sep().next()), None);
        assert_eq!(
            amount_in(&chain, sep().next().next()),
            Some(money("275.00"))
        );
    }

    #[test]
    fn an_explicit_zero_is_budgeted_and_no_amount_is_not() {
        let chain = [onward(sep(), "0.00")];
        assert_eq!(amount_in(&chain, sep()), Some(money("0.00")));
        assert_eq!(amount_in(&chain, sep().prev()), None);
    }

    #[test]
    fn a_record_marks_its_own_month_for_the_bold_cell() {
        let chain = [onward(sep().prev(), "250.00")];
        assert_eq!(
            applied(&chain, sep().prev()).map(|a| a.own_record),
            Some(true)
        );
        assert_eq!(applied(&chain, sep()).map(|a| a.own_record), Some(false));
    }

    // -- effective budget and carry ---------------------------------------------------------------

    fn unreachable_budget() -> Budget {
        Budget {
            id: 0,
            name: String::new(),
            method: Method::Limits,
            unit: String::new(),
            account_ids: Vec::new(),
            is_default: false,
            archived_at: None,
            limits: BTreeMap::new(),
        }
    }

    /// A Budget whose Electricity chain starts in July with the given Rollover.
    fn electricity_budget(world: &World, rollover: Rollover) -> Budget {
        let mut budget = world
            .budgets()
            .0
            .get(1)
            .cloned()
            .unwrap_or_else(unreachable_budget);
        budget.limits.insert(
            world.category("Electricity"),
            vec![LimitRecord {
                month: Period::of(date(2026, 7, 1)),
                limit: Limit::Onward {
                    amount: money("90.00"),
                    rollover,
                },
            }],
        );
        budget
    }

    fn carried_into_september(world: &World, rollover: Rollover) -> Option<Money> {
        let budget = electricity_budget(world, rollover);
        effective_budget(
            &budget,
            &world.ledger(),
            world.category("Electricity"),
            sep(),
            today(),
        )
        .map(|e| e.carried_in)
    }

    #[test]
    fn carry_unspent_compounds_across_closed_months() {
        let mut world = World::new();
        world.post("ANZ Everyday", "Electricity", date(2026, 7, 10), "-60.00");
        world.post("ANZ Everyday", "Electricity", date(2026, 8, 10), "-70.00");
        // July leaves 30; August is 90 + 30 - 70 = 50.
        assert_eq!(
            carried_into_september(&world, Rollover::CarryUnspent),
            Some(money("50.00"))
        );
    }

    #[test]
    fn carry_unspent_ignores_an_overspend_but_carry_both_takes_it() {
        let mut world = World::new();
        world.post("ANZ Everyday", "Electricity", date(2026, 7, 10), "-120.00");
        world.post("ANZ Everyday", "Electricity", date(2026, 8, 10), "-90.00");
        assert_eq!(
            carried_into_september(&world, Rollover::CarryUnspent),
            Some(money("0.00"))
        );
        // July overspends by 30, so August's budget is 60 and leaves -30 more: -30 carried.
        assert_eq!(
            carried_into_september(&world, Rollover::CarryBoth),
            Some(money("-30.00"))
        );
    }

    #[test]
    fn no_rollover_carries_nothing() {
        let mut world = World::new();
        world.post("ANZ Everyday", "Electricity", date(2026, 7, 10), "-10.00");
        assert_eq!(
            carried_into_september(&world, Rollover::None),
            Some(money("0.00"))
        );
    }

    #[test]
    fn the_current_month_never_carries_into_next_month() {
        let world = World::new();
        let budget = electricity_budget(&world, Rollover::CarryBoth);
        let next = effective_budget(
            &budget,
            &world.ledger(),
            world.category("Electricity"),
            sep().next(),
            today(),
        );
        assert_eq!(next.map(|e| e.carried_in), Some(money("0.00")));
    }

    #[test]
    fn a_stop_or_an_unbudgeted_month_resets_the_carry() {
        let mut world = World::new();
        world.post("ANZ Everyday", "Electricity", date(2026, 7, 10), "-10.00");
        let mut budget = electricity_budget(&world, Rollover::CarryUnspent);
        let electricity = world.category("Electricity");
        // July leaves 80; a Stop in August resets it, and September resumes from nothing.
        let august = Period::of(date(2026, 8, 1));
        if let Some(chain) = budget.limits.get_mut(&electricity) {
            put(chain, stop(august));
            put(
                chain,
                LimitRecord {
                    month: sep(),
                    limit: Limit::Onward {
                        amount: money("90.00"),
                        rollover: Rollover::CarryUnspent,
                    },
                },
            );
        }
        let found = effective_budget(&budget, &world.ledger(), electricity, sep(), today());
        assert_eq!(found.map(|e| e.carried_in), Some(money("0.00")));
    }

    #[test]
    fn a_month_only_amount_carries_on_its_own_rollover() {
        let mut world = World::new();
        world.post("ANZ Everyday", "Electricity", date(2026, 8, 10), "-40.00");
        let mut budget = electricity_budget(&world, Rollover::None);
        let electricity = world.category("Electricity");
        let august = Period::of(date(2026, 8, 1));
        if let Some(chain) = budget.limits.get_mut(&electricity) {
            put(
                chain,
                LimitRecord {
                    month: august,
                    limit: Limit::MonthOnly {
                        amount: money("100.00"),
                        rollover: Rollover::CarryUnspent,
                    },
                },
            );
        }
        let found = effective_budget(&budget, &world.ledger(), electricity, sep(), today());
        assert_eq!(found.map(|e| e.carried_in), Some(money("60.00")));
    }

    // -- period figures ---------------------------------------------------------------------------

    /// Dining budgeted 250 and Groceries 500 from September; Dining overspent by 68.40.
    fn september() -> (World, Budgets, u32) {
        let mut world = World::new();
        world.post("ANZ Everyday", "Dining", date(2026, 9, 3), "-200.00");
        world.post("ANZ Offset", "Dining", date(2026, 9, 12), "-118.40");
        world.post("ANZ Everyday", "Groceries", date(2026, 9, 5), "-120.00");
        world.post("ANZ Everyday", "Groceries", date(2026, 9, 6), "20.00");
        world.post("ANZ Everyday", "Household", date(2026, 9, 7), "-58.00");
        // Off budget: the Amex isn't on this Budget.
        world.post("Amex Platinum", "Groceries", date(2026, 9, 8), "-999.00");
        // An earlier month never lands in September.
        world.post("ANZ Everyday", "Dining", date(2026, 8, 30), "-77.00");
        let (mut budgets, id) = world.budgets();
        set(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep(),
            Span::Onward,
            "250.00",
            None,
        );
        set(
            &mut budgets,
            &world,
            id,
            "Groceries",
            sep(),
            Span::Onward,
            "500.00",
            None,
        );
        (world, budgets, id)
    }

    #[test]
    fn spent_is_signed_and_counts_only_on_budget_accounts() {
        let (world, budgets, id) = september();
        let figures = period_figures(
            &budgets.get(id).cloned().unwrap_or_else(unreachable_budget),
            &world.ledger(),
            sep(),
            today(),
        );
        let groceries = row(&figures, world.category("Groceries"));
        // 120 spent, 20 refunded, the Amex purchase off budget.
        assert_eq!(groceries.map(|r| r.spent.clone()), Some(money("100.00")));
        let dining = row(&figures, world.category("Dining"));
        assert_eq!(dining.map(|r| r.spent.clone()), Some(money("318.40")));
    }

    #[test]
    fn a_leaf_over_its_budget_counts_and_carries_the_negative_left() {
        let (world, budgets, id) = september();
        let figures = period_figures(
            &budgets.get(id).cloned().unwrap_or_else(unreachable_budget),
            &world.ledger(),
            sep(),
            today(),
        );
        let dining = row(&figures, world.category("Dining"));
        assert_eq!(dining.map(|r| r.over), Some(true));
        assert_eq!(dining.and_then(|r| r.left.clone()), Some(money("-68.40")));
        assert_eq!(figures.over_count, 1);
    }

    #[test]
    fn totals_reconcile_left_with_budgeted_spent_and_known() {
        let (world, budgets, id) = september();
        let figures = period_figures(
            &budgets.get(id).cloned().unwrap_or_else(unreachable_budget),
            &world.ledger(),
            sep(),
            today(),
        );
        assert_eq!(figures.budgeted, money("750.00"));
        assert_eq!(figures.spent, money("418.40"));
        assert_eq!(figures.known, money("0.00"));
        assert_eq!(figures.left, money("331.60"));
    }

    #[test]
    fn unbudgeted_spending_is_listed_but_not_rolled_up_or_counted() {
        let (world, budgets, id) = september();
        let figures = period_figures(
            &budgets.get(id).cloned().unwrap_or_else(unreachable_budget),
            &world.ledger(),
            sep(),
            today(),
        );
        let household = row(&figures, world.category("Household"));
        assert_eq!(household.map(|r| r.budget.clone()), Some(None));
        assert_eq!(household.map(|r| r.spent.clone()), Some(money("58.00")));
        assert_eq!(figures.unbudgeted_spent, money("58.00"));
        assert_eq!(figures.over_count, 1);
    }

    #[test]
    fn a_parent_rolls_up_only_its_budgeted_leaves() {
        let (world, mut budgets, id) = september();
        set(
            &mut budgets,
            &world,
            id,
            "Transport",
            sep(),
            Span::Onward,
            "100.00",
            None,
        );
        let figures = period_figures(
            &budgets.get(id).cloned().unwrap_or_else(unreachable_budget),
            &world.ledger(),
            sep(),
            today(),
        );
        let food = row(&figures, world.category("Food"));
        assert_eq!(food.map(|r| r.is_parent), Some(true));
        assert_eq!(food.and_then(|r| r.budget.clone()), Some(money("750.00")));
        assert_eq!(food.map(|r| r.spent.clone()), Some(money("418.40")));
    }

    #[test]
    fn a_parent_leaves_an_unbudgeted_childs_spending_out_of_its_spent() {
        let mut world = World::new();
        world.post("ANZ Everyday", "Groceries", date(2026, 9, 5), "-50.00");
        world.post("ANZ Everyday", "Dining", date(2026, 9, 6), "-30.00");
        let (mut budgets, id) = world.budgets();
        set(
            &mut budgets,
            &world,
            id,
            "Groceries",
            sep(),
            Span::Onward,
            "200.00",
            None,
        );
        let figures = period_figures(
            &budgets.get(id).cloned().unwrap_or_else(unreachable_budget),
            &world.ledger(),
            sep(),
            today(),
        );
        let food = row(&figures, world.category("Food"));
        assert_eq!(food.map(|r| r.spent.clone()), Some(money("50.00")));
        let dining = row(&figures, world.category("Dining"));
        assert_eq!(dining.map(|r| r.budget.clone()), Some(None));
        assert_eq!(dining.map(|r| r.spent.clone()), Some(money("30.00")));
    }

    #[test]
    fn a_zero_budget_makes_any_spend_over() {
        let mut world = World::new();
        world.post("ANZ Everyday", "Groceries", date(2026, 9, 5), "-1.00");
        let (mut budgets, id) = world.budgets();
        set(
            &mut budgets,
            &world,
            id,
            "Groceries",
            sep(),
            Span::Onward,
            "0.00",
            None,
        );
        let figures = period_figures(
            &budgets.get(id).cloned().unwrap_or_else(unreachable_budget),
            &world.ledger(),
            sep(),
            today(),
        );
        assert_eq!(
            row(&figures, world.category("Groceries")).map(|r| r.over),
            Some(true)
        );
    }

    #[test]
    fn known_costs_are_current_due_and_carried_overdue_on_budget_accounts() {
        let mut world = World::new();
        // Due later this month, overdue earlier this month, overdue in August, and next month.
        world.bill("ANZ Everyday", "Electricity", date(2026, 9, 28), "80.00");
        world.bill("ANZ Everyday", "Electricity", date(2026, 9, 10), "20.00");
        world.bill("ANZ Everyday", "Electricity", date(2026, 8, 15), "5.00");
        world.bill("ANZ Everyday", "Electricity", date(2026, 10, 3), "9.00");
        // A different Account that isn't on this Budget.
        world.bill("Amex Platinum", "Electricity", date(2026, 9, 25), "1000.00");
        let (mut budgets, id) = world.budgets();
        set(
            &mut budgets,
            &world,
            id,
            "Electricity",
            sep(),
            Span::Onward,
            "200.00",
            None,
        );
        let budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
        let electricity = world.category("Electricity");

        let now = period_figures(&budget, &world.ledger(), sep(), today());
        assert_eq!(
            row(&now, electricity).map(|r| r.known.clone()),
            Some(money("105.00"))
        );
        assert_eq!(now.left, money("95.00"));
        // The four unpaid bills on the Budget's Account: two this month, one carried, none off-budget.
        assert_eq!(now.known_count, 3);

        let next = period_figures(&budget, &world.ledger(), sep().next(), today());
        assert_eq!(
            row(&next, electricity).map(|r| r.known.clone()),
            Some(money("9.00"))
        );

        let past = period_figures(&budget, &world.ledger(), sep().prev(), today());
        assert!(row(&past, electricity).is_none());
    }

    #[test]
    fn a_paid_bill_stops_counting_once_it_is_no_longer_open() {
        let mut world = World::new();
        world.bill("ANZ Everyday", "Electricity", date(2026, 9, 28), "80.00");
        if let Some(entry) = world.entries.first_mut() {
            entry.resolution = bills::Resolution::Skipped {
                planned: money("80.00"),
            };
        }
        let (mut budgets, id) = world.budgets();
        set(
            &mut budgets,
            &world,
            id,
            "Electricity",
            sep(),
            Span::Onward,
            "200.00",
            None,
        );
        let figures = period_figures(
            &budgets.get(id).cloned().unwrap_or_else(unreachable_budget),
            &world.ledger(),
            sep(),
            today(),
        );
        assert_eq!(figures.known, money("0.00"));
    }

    #[test]
    fn known_costs_alone_pushing_a_leaf_over_is_at_risk_and_not_counted_as_over() {
        let mut world = World::new();
        world.post("ANZ Everyday", "Electricity", date(2026, 9, 5), "-60.00");
        world.bill("ANZ Everyday", "Electricity", date(2026, 9, 28), "80.00");
        let (mut budgets, id) = world.budgets();
        set(
            &mut budgets,
            &world,
            id,
            "Electricity",
            sep(),
            Span::Onward,
            "100.00",
            None,
        );
        let figures = period_figures(
            &budgets.get(id).cloned().unwrap_or_else(unreachable_budget),
            &world.ledger(),
            sep(),
            today(),
        );
        let row = row(&figures, world.category("Electricity"));
        assert_eq!(row.map(|r| (r.over, r.at_risk)), Some((false, true)));
        assert_eq!((figures.over_count, figures.at_risk_count), (0, 1));
        assert_eq!(row.and_then(|r| r.left.clone()), Some(money("-40.00")));
    }

    #[test]
    fn known_costs_without_a_budget_amount_are_listed_as_unbudgeted() {
        let mut world = World::new();
        world.bill("ANZ Everyday", "Water", date(2026, 9, 28), "40.00");
        let (budgets, id) = world.budgets();
        let figures = period_figures(
            &budgets.get(id).cloned().unwrap_or_else(unreachable_budget),
            &world.ledger(),
            sep(),
            today(),
        );
        assert_eq!(
            row(&figures, world.category("Water")).map(|r| r.known.clone()),
            Some(money("40.00"))
        );
        assert_eq!(figures.unbudgeted_known, money("40.00"));
        assert_eq!(figures.known, money("0.00"));
    }

    #[test]
    fn elapsed_and_per_day_use_days_remaining_including_today() {
        let (world, budgets, id) = september();
        let budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
        let now = period_figures(&budget, &world.ledger(), sep(), today());
        // Day 21 of 30: 70% through, 10 days left including today.
        assert_eq!(
            now.elapsed,
            Elapsed {
                day: 21,
                days_in_month: 30,
                percent: 70,
                days_left: 10
            }
        );
        assert_eq!(now.per_day, Some(money("33.16")));

        let past = period_figures(&budget, &world.ledger(), sep().prev(), today());
        assert_eq!((past.elapsed.percent, past.elapsed.days_left), (100, 0));
        assert_eq!(past.per_day, None);

        let future = period_figures(&budget, &world.ledger(), sep().next(), today());
        assert_eq!((future.elapsed.percent, future.elapsed.days_left), (0, 31));
    }

    #[test]
    fn health_summarises_the_current_month() {
        let (world, budgets, id) = september();
        let health = health(
            &budgets.get(id).cloned().unwrap_or_else(unreachable_budget),
            &world.ledger(),
            today(),
        );
        assert_eq!(
            health,
            Health {
                over: 1,
                left: money("331.60"),
                at_risk: 0
            }
        );
    }

    // -- history ------------------------------------------------------------------------------------

    fn history_world() -> (World, Budget) {
        let mut world = World::new();
        // June under, July over, August under, September so far over.
        world.post("ANZ Everyday", "Dining", date(2026, 6, 4), "-100.00");
        world.post("ANZ Everyday", "Dining", date(2026, 7, 4), "-350.00");
        world.post("ANZ Everyday", "Dining", date(2026, 8, 4), "-200.00");
        world.post("ANZ Everyday", "Dining", date(2026, 9, 4), "-400.00");
        let mut budget = world
            .budgets()
            .0
            .get(1)
            .cloned()
            .unwrap_or_else(unreachable_budget);
        budget.limits.insert(
            world.category("Dining"),
            vec![LimitRecord {
                month: Period::of(date(2026, 6, 1)),
                limit: Limit::Onward {
                    amount: money("300.00"),
                    rollover: Rollover::None,
                },
            }],
        );
        (world, budget)
    }

    #[test]
    fn history_average_and_over_use_closed_months_only() {
        let (world, budget) = history_world();
        let first = Period::of(date(2026, 6, 1));
        let history = history(&budget, &world.ledger(), first, sep(), today());
        let dining = history
            .rows
            .iter()
            .find(|r| r.category_id == world.category("Dining"));
        // June 100, July 350, August 200: mean 216.67; over in July only; September counts in neither.
        assert_eq!(
            dining.and_then(|r| r.average.clone()),
            Some(money("216.67"))
        );
        assert_eq!(
            dining.map(|r| (r.over_months, r.closed_months)),
            Some((1, 3))
        );
        assert_eq!(dining.map(|r| r.cells.len()), Some(4));
        let september = dining.and_then(|r| r.cells.last().cloned()).flatten();
        assert_eq!(september.map(|c| c.over), Some(true));
    }

    #[test]
    fn history_shows_a_dash_for_an_unbudgeted_month_and_parents_as_rollups() {
        let (world, budget) = history_world();
        let earlier = Period::of(date(2026, 5, 1));
        let history = history(&budget, &world.ledger(), earlier, sep(), today());
        let dining = history
            .rows
            .iter()
            .find(|r| r.category_id == world.category("Dining"));
        assert_eq!(dining.map(|r| r.cells[0].is_none()), Some(true));
        let food = history
            .rows
            .iter()
            .find(|r| r.category_id == world.category("Food"));
        assert_eq!(food.map(|r| r.is_parent), Some(true));
        assert_eq!(
            food.and_then(|r| r.cells[2].clone()).map(|c| c.spent),
            Some(money("350.00"))
        );
        assert_eq!((history.leaves_shown, history.leaves_ever_budgeted), (1, 1));
    }

    #[test]
    fn history_month_totals_count_budgeted_leaves_only_and_mark_the_current_month() {
        let (mut world, budget) = history_world();
        world.post("ANZ Everyday", "Household", date(2026, 7, 9), "-999.00");
        let first = Period::of(date(2026, 6, 1));
        let history = history(&budget, &world.ledger(), first, sep(), today());
        let july = &history.months[1];
        assert_eq!(
            (july.budget.clone(), july.spent.clone(), july.over),
            (money("300.00"), money("350.00"), true)
        );
        assert_eq!(
            history
                .months
                .iter()
                .map(|m| m.is_current)
                .collect::<Vec<_>>(),
            [false, false, false, true]
        );
    }

    #[test]
    fn history_bounds_run_from_the_first_amount_to_the_current_month() {
        let (_, budget) = history_world();
        assert_eq!(
            history_bounds(&budget, today()),
            Some((Period::of(date(2026, 6, 1)), sep()))
        );
        assert_eq!(history_bounds(&unreachable_budget(), today()), None);
    }

    #[test]
    fn the_history_range_is_six_months_inside_the_bounds() {
        let (_, budget) = history_world();
        let jun = Period::of(date(2026, 6, 1));
        // The Budget starts in June, so only four months exist yet.
        assert_eq!(history_range(&budget, sep(), today()), Some((jun, sep())));
        assert_eq!(
            history_range(&budget, sep().next(), today()),
            Some((jun, sep())),
            "never past the current month"
        );
        assert_eq!(
            history_range(&budget, jun, today()),
            Some((jun, sep())),
            "never a shorter window than the Budget allows"
        );
        // A later today gives a full window that can step back.
        let later = date(2027, 3, 10);
        let mar = Period::of(later);
        assert_eq!(
            history_range(&budget, mar, later),
            Some((Period::of(date(2026, 10, 1)), mar))
        );
        assert_eq!(
            history_range(&budget, Period::of(date(2026, 12, 1)), later),
            Some((Period::of(date(2026, 7, 1)), Period::of(date(2026, 12, 1))))
        );
        assert_eq!(history_range(&unreachable_budget(), sep(), today()), None);
    }

    #[test]
    fn the_history_csv_writes_plain_numbers_and_quotes_awkward_names() {
        let (mut world, budget) = history_world();
        let dining = world.category("Dining");
        if let Some(category) = world.categories.iter_mut().find(|c| c.id == dining) {
            category.name = "Dining, \"out\"".to_string();
        }
        let jun = Period::of(date(2026, 6, 1));
        let shown = history(&budget, &world.ledger(), jun, sep(), today());
        let labels = HistoryCsvLabels {
            category: "Category".into(),
            budget: "budget".into(),
            spent: "spent".into(),
            average: "Avg".into(),
            over: "Over".into(),
        };
        let csv = history_csv(&shown, &world.categories, &labels, |month| {
            format!("{}-{:02}", month.year, month.month)
        });
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines.len(), shown.rows.len() + 1);
        assert!(lines[0].starts_with("Category,2026-06 budget,2026-06 spent,"));
        assert!(lines[0].ends_with(",2026-09 budget,2026-09 spent,Avg,Over"));
        let row = shown
            .rows
            .iter()
            .position(|row| row.category_id == dining)
            .unwrap_or_else(|| unreachable!("Dining is budgeted"));
        let line = lines[row + 1];
        assert!(
            line.contains("Dining, \"\"out\"\"\",300.00,100.00,300.00,350.00,"),
            "{line}"
        );
        assert!(line.starts_with('"'));
        assert!(!line.contains('\u{25b2}'));
        let dining_row = &shown.rows[row];
        assert!(line.ends_with(&format!(
            ",{}/{}",
            dining_row.over_months, dining_row.closed_months
        )));
        // Every line has the same number of unquoted separators as the header has columns.
        assert_eq!(lines[0].split(',').count(), 1 + shown.months.len() * 2 + 2);
    }

    // -- averages and Unallocated --------------------------------------------------------------------

    #[test]
    fn the_three_month_average_rounds_to_the_nearest_ten_and_floors_at_zero() {
        let mut world = World::new();
        world.post("ANZ Everyday", "Dining", date(2026, 6, 4), "-100.00");
        world.post("ANZ Everyday", "Dining", date(2026, 7, 4), "-100.00");
        world.post("ANZ Everyday", "Dining", date(2026, 8, 4), "-113.00");
        // Mean 104.33 rounds to 100.00. September itself is excluded.
        world.post("ANZ Everyday", "Dining", date(2026, 9, 4), "-900.00");
        world.post("ANZ Everyday", "Groceries", date(2026, 8, 4), "20.00");
        let (budgets, id) = world.budgets();
        let budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
        let ledger = world.ledger();
        assert_eq!(
            three_month_average(&budget, &ledger, world.category("Dining"), sep()),
            money("100.00")
        );
        // A net refund averages below zero and is floored.
        assert_eq!(
            three_month_average(&budget, &ledger, world.category("Groceries"), sep()),
            money("0.00")
        );
    }

    // -- the Dashboard's bars -------------------------------------------------------------------

    #[test]
    fn dashboard_bars_take_the_fullest_budgeted_top_level_rows() {
        let mut world = World::new();
        world.post("ANZ Everyday", "Dining", date(2026, 9, 4), "-300.00");
        world.post("ANZ Everyday", "Groceries", date(2026, 9, 5), "-100.00");
        world.post("ANZ Everyday", "Rent", date(2026, 9, 1), "-900.00");
        world.post("ANZ Everyday", "Transport", date(2026, 9, 2), "-30.00");
        world.post("ANZ Everyday", "Household", date(2026, 9, 3), "-10.00");
        let (mut budgets, id) = world.budgets();
        for (name, amount) in [
            ("Dining", "200.00"),
            ("Groceries", "600.00"),
            ("Rent", "1000.00"),
            ("Transport", "600.00"),
            ("Water", "50.00"),
        ] {
            let set = budgets.set_amount(
                id,
                &world.categories,
                world.category(name),
                sep(),
                Span::Onward,
                money(amount),
                None,
                today(),
            );
            assert_eq!(set, Ok(()), "{name}");
        }
        let budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
        let figures = period_figures(&budget, &world.ledger(), sep(), today());
        let bars = dashboard_bars(&figures, &world.categories);
        assert!(bars.len() <= DASHBOARD_BARS);
        for bar in &bars {
            assert_eq!(categories::depth(&world.categories, bar.category_id), 0);
        }
        // Rent (90%) leads, as its top-level parent's rollup; Household is unbudgeted and so is
        // never shown.
        let top_of = |name: &str| {
            let id = world.category(name);
            world
                .categories
                .iter()
                .find(|category| category.id == id)
                .and_then(|category| category.parent)
                .unwrap_or(id)
        };
        assert_eq!(
            bars.first().map(|bar| bar.category_id),
            Some(top_of("Rent"))
        );
        assert!(
            bars.iter()
                .all(|bar| bar.category_id != top_of("Household"))
        );
        // Food rolls Dining (over) and Groceries up to 400 of 800: not over as a whole.
        let food = top_of("Dining");
        let food_bar = bars.iter().find(|bar| bar.category_id == food);
        assert_eq!(food_bar.map(|bar| bar.spent.clone()), Some(money("400.00")));
        assert_eq!(food_bar.map(|bar| bar.over), Some(false));
    }

    // -- the Switcher -------------------------------------------------------------------------------

    #[test]
    fn the_switcher_lists_active_budgets_then_archived_and_filters_by_name() {
        let world = World::new();
        let mut budgets = default_budgets(&world.accounts, &world.categories, today());
        let ids: Vec<u32> = budgets.all().iter().map(|b| b.id).collect();
        assert!(ids.len() >= 2, "the seed has more than one Budget");
        assert_eq!(switcher_ids(&budgets, ""), ids);

        // Archiving moves a Budget below the active ones.
        let other = ids
            .iter()
            .copied()
            .find(|id| *id != PERSONAL_SPENDING_ID)
            .unwrap_or_default();
        assert_eq!(budgets.archive(other, today()), Ok(()));
        assert_eq!(switcher_ids(&budgets, "").last(), Some(&other));

        assert_eq!(
            switcher_ids(&budgets, "  PERSONAL "),
            vec![PERSONAL_SPENDING_ID]
        );
        assert!(switcher_ids(&budgets, "no such budget").is_empty());
    }

    #[test]
    fn the_switcher_opens_on_the_current_budget_and_steps_inside_its_rows() {
        let world = World::new();
        let budgets = default_budgets(&world.accounts, &world.categories, today());
        let ids = switcher_ids(&budgets, "");
        let last = *ids.last().unwrap_or(&0);
        let mut switcher = Switcher::new(&budgets, last);
        assert_eq!(switcher.chosen(), Some(last));
        switcher.step(true);
        assert_eq!(switcher.chosen(), Some(last), "stops at the end");
        for _ in 0..ids.len() {
            switcher.step(false);
        }
        assert_eq!(switcher.chosen(), ids.first().copied());

        // Typing narrows the rows and puts the highlight back on the first.
        switcher.step(true);
        for c in "personal".chars() {
            switcher.type_char(c);
        }
        assert_eq!(switcher.chosen(), Some(PERSONAL_SPENDING_ID));
        switcher.type_char('z');
        assert_eq!(switcher.chosen(), None);
        switcher.backspace();
        assert_eq!(switcher.chosen(), Some(PERSONAL_SPENDING_ID));
    }

    // -- Fill ---------------------------------------------------------------------------------------

    #[test]
    fn fill_targets_the_first_open_month_at_or_after_the_cursor() {
        assert_eq!(fill_target(Some(sep()), today()), sep());
        assert_eq!(
            fill_target(Some(sep().next().next()), today()),
            sep().next().next()
        );
        assert_eq!(fill_target(Some(sep().prev()), today()), sep().next());
        assert_eq!(fill_target(None, today()), sep().next());
    }

    /// Dining 250 Onward with a Month-only 400 in September, Groceries 500 Onward, Rent 1000 with
    /// its own October record, Transport budgeted from October only.
    fn fill_world() -> (World, Budgets, u32) {
        let mut world = World::new();
        for month in [6, 7, 8] {
            world.post("ANZ Everyday", "Dining", date(2026, month, 4), "-312.00");
            world.post("ANZ Everyday", "Groceries", date(2026, month, 4), "-500.00");
        }
        let (mut budgets, id) = world.budgets();
        let categories = world.categories.clone();
        let jun = Period::of(date(2026, 6, 1));
        let mut set = |name: &str, month: Period, span: Span, amount: &str| {
            budgets
                .set_amount(
                    id,
                    &categories,
                    category_id(&categories, name),
                    month,
                    span,
                    money(amount),
                    Some(Rollover::CarryUnspent),
                    // Seeding history: every month is open as of June.
                    date(2026, 6, 1),
                )
                .expect("a seeded amount");
        };
        set("Dining", jun, Span::Onward, "250.00");
        set("Dining", sep(), Span::MonthOnly, "400.00");
        set("Groceries", jun, Span::Onward, "500.00");
        set("Rent", jun, Span::Onward, "1000.00");
        set("Rent", sep().next(), Span::Onward, "1100.00");
        set("Transport", sep().next(), Span::Onward, "60.00");
        (world, budgets, id)
    }

    #[test]
    fn fill_from_the_previous_month_copies_amounts_and_keeps_edited_cells() {
        let (world, mut budgets, id) = fill_world();
        let october = sep().next();
        let budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
        let preview = fill_preview(&budget, &world.ledger(), october, FillSource::PreviousMonth);
        // Dining's September Month-only differs from the plan; Groceries matches it; Rent has its
        // own October record; Transport wasn't budgeted in September.
        assert_eq!(
            preview.changes,
            vec![FillLine {
                category_id: world.category("Dining"),
                before: Some(money("250.00")),
                after: money("400.00"),
            }]
        );
        assert_eq!(preview.kept, vec![world.category("Rent")]);
        assert_eq!(preview.unchanged, 1);
        assert_eq!(preview.total, money("2060.00"));

        let wrote = budgets.fill(
            id,
            &world.ledger(),
            october,
            FillSource::PreviousMonth,
            today(),
        );
        assert_eq!(wrote, Ok(1));
        let budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
        let dining = budget.chain(world.category("Dining"));
        let filled = applied(dining, october).unwrap_or_else(|| unreachable!("filled"));
        assert_eq!(filled.amount, money("400.00"));
        assert_eq!(filled.rollover, Rollover::CarryUnspent);
        // Month-only: November falls back to the Onward amount.
        assert_eq!(
            applied(dining, october.next()).map(|found| found.amount),
            Some(money("250.00"))
        );
        assert_eq!(
            total_budgeted(&budget, &world.categories, october),
            preview.total
        );
        // A second Fill finds every Category kept or unchanged.
        assert_eq!(
            budgets.fill(
                id,
                &world.ledger(),
                october,
                FillSource::PreviousMonth,
                today()
            ),
            Ok(0)
        );
    }

    #[test]
    fn fill_from_the_average_rounds_spent_and_refuses_a_closed_month() {
        let (world, mut budgets, id) = fill_world();
        let budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
        // The current month: the three months before it are June to August.
        let preview = fill_preview(&budget, &world.ledger(), sep(), FillSource::Average);
        // Dining has its own September record and Rent has no spend: 1000.00 becomes 0.00.
        assert_eq!(preview.kept, vec![world.category("Dining")]);
        assert_eq!(
            preview.changes,
            vec![FillLine {
                category_id: world.category("Rent"),
                before: Some(money("1000.00")),
                after: money("0.00"),
            }]
        );
        assert_eq!(preview.unchanged, 1, "Groceries averages its 500.00");
        assert_eq!(
            budgets.fill(
                id,
                &world.ledger(),
                sep().prev(),
                FillSource::Average,
                today()
            ),
            Err(BudgetError::ClosedMonth)
        );
    }

    #[test]
    fn an_average_exactly_on_a_five_rounds_away_from_zero() {
        let mut world = World::new();
        world.post("ANZ Everyday", "Dining", date(2026, 8, 4), "-45.00");
        // Total 45 over 3 months is 15.00, which rounds up to 20.
        let (budgets, id) = world.budgets();
        let budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
        assert_eq!(
            three_month_average(&budget, &world.ledger(), world.category("Dining"), sep()),
            money("20.00")
        );
    }

    #[test]
    fn unallocated_is_average_income_minus_total_budgeted() {
        let mut world = World::new();
        world.post("ANZ Everyday", "Salary", date(2026, 6, 25), "6000.00");
        world.post("ANZ Everyday", "Salary", date(2026, 7, 25), "6000.00");
        world.post("ANZ Everyday", "Salary", date(2026, 8, 25), "5550.00");
        // Off-budget income doesn't count.
        world.post("Amex Platinum", "Salary", date(2026, 8, 26), "3000.00");
        let (mut budgets, id) = world.budgets();
        set(
            &mut budgets,
            &world,
            id,
            "Rent",
            sep(),
            Span::Onward,
            "1000.00",
            None,
        );
        set(
            &mut budgets,
            &world,
            id,
            "Groceries",
            sep(),
            Span::Onward,
            "5000.00",
            None,
        );
        let budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
        let ledger = world.ledger();
        assert_eq!(average_income(&budget, &ledger, today()), money("5850.00"));
        assert_eq!(
            total_budgeted(&budget, &world.categories, sep()),
            money("6000.00")
        );
        assert_eq!(
            unallocated(&budget, &ledger, sep(), today()),
            money("-150.00")
        );
    }

    // -- operations ------------------------------------------------------------------------------------

    #[test]
    fn saving_replaces_the_record_that_starts_in_that_month() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        set(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep(),
            Span::Onward,
            "250.00",
            None,
        );
        set(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep().next(),
            Span::Onward,
            "300.00",
            None,
        );
        set(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep().next(),
            Span::MonthOnly,
            "320.00",
            None,
        );
        let chain = budgets
            .get(id)
            .map(|b| b.chain(world.category("Dining")).to_vec())
            .unwrap_or_default();
        assert_eq!(chain.len(), 2);
        assert_eq!(amount_in(&chain, sep().next()), Some(money("320.00")));
        assert_eq!(
            amount_in(&chain, sep().next().next()),
            Some(money("250.00"))
        );
    }

    #[test]
    fn a_new_record_inherits_the_rollover_of_the_one_it_follows() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        set(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep(),
            Span::Onward,
            "250.00",
            Some(Rollover::CarryBoth),
        );
        set(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep().next(),
            Span::Onward,
            "300.00",
            None,
        );
        let chain = budgets
            .get(id)
            .map(|b| b.chain(world.category("Dining")).to_vec())
            .unwrap_or_default();
        assert_eq!(
            applied(&chain, sep().next()).map(|a| a.rollover),
            Some(Rollover::CarryBoth)
        );
        // With nothing before it, it starts at None.
        set(
            &mut budgets,
            &world,
            id,
            "Water",
            sep(),
            Span::Onward,
            "60.00",
            None,
        );
        let water = budgets
            .get(id)
            .map(|b| b.chain(world.category("Water")).to_vec())
            .unwrap_or_default();
        assert_eq!(
            applied(&water, sep()).map(|a| a.rollover),
            Some(Rollover::None)
        );
    }

    #[test]
    fn only_leaf_expense_categories_hold_amounts() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        for name in ["Food", "Salary"] {
            let refused = budgets.set_amount(
                id,
                &world.categories,
                world.category(name),
                sep(),
                Span::Onward,
                money("10.00"),
                None,
                today(),
            );
            assert_eq!(refused, Err(BudgetError::NotAnExpenseLeaf), "{name}");
        }
    }

    #[test]
    fn closed_months_are_read_only_and_amounts_cant_be_negative() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        let dining = world.category("Dining");
        let closed = budgets.set_amount(
            id,
            &world.categories,
            dining,
            sep().prev(),
            Span::Onward,
            money("1.00"),
            None,
            today(),
        );
        assert_eq!(closed, Err(BudgetError::ClosedMonth));
        let negative = budgets.set_amount(
            id,
            &world.categories,
            dining,
            sep(),
            Span::Onward,
            money("-1.00"),
            None,
            today(),
        );
        assert_eq!(negative, Err(BudgetError::NegativeAmount));
        assert_eq!(
            budgets.stop(id, &world.categories, dining, sep().prev(), today()),
            Err(BudgetError::ClosedMonth)
        );
    }

    #[test]
    fn stopping_ends_the_amount_from_that_month_and_keeps_the_past() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        set(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep(),
            Span::Onward,
            "250.00",
            None,
        );
        assert_eq!(
            budgets.stop(
                id,
                &world.categories,
                world.category("Dining"),
                sep().next(),
                today()
            ),
            Ok(())
        );
        let chain = budgets
            .get(id)
            .map(|b| b.chain(world.category("Dining")).to_vec())
            .unwrap_or_default();
        assert_eq!(amount_in(&chain, sep()), Some(money("250.00")));
        assert_eq!(amount_in(&chain, sep().next()), None);
    }

    #[test]
    fn stopping_a_category_with_no_amount_writes_nothing() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        assert_eq!(
            budgets.stop(
                id,
                &world.categories,
                world.category("Dining"),
                sep(),
                today()
            ),
            Ok(())
        );
        assert_eq!(budgets.get(id).map(|b| b.category_ids().count()), Some(0));
    }

    #[test]
    fn clearing_a_month_only_record_restores_the_carried_onward_value() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        let dining = world.category("Dining");
        set(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep(),
            Span::Onward,
            "250.00",
            None,
        );
        set(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep().next(),
            Span::MonthOnly,
            "400.00",
            None,
        );
        assert_eq!(
            budgets.clear_month_only(id, dining, sep().next(), today()),
            Ok(true)
        );
        // The Onward record itself is not a Month-only, so nothing more comes off.
        assert_eq!(
            budgets.clear_month_only(id, dining, sep(), today()),
            Ok(false)
        );
        let chain = budgets
            .get(id)
            .map(|b| b.chain(dining).to_vec())
            .unwrap_or_default();
        assert_eq!(amount_in(&chain, sep().next()), Some(money("250.00")));
    }

    #[test]
    fn cycling_the_rollover_writes_an_onward_from_the_current_month() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        let dining = world.category("Dining");
        set(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep(),
            Span::MonthOnly,
            "250.00",
            None,
        );
        assert_eq!(
            budgets.cycle_rollover(id, dining, today()),
            Ok(Rollover::CarryUnspent)
        );
        assert_eq!(
            budgets.cycle_rollover(id, dining, today()),
            Ok(Rollover::CarryBoth)
        );
        assert_eq!(
            budgets.cycle_rollover(id, dining, today()),
            Ok(Rollover::None)
        );
        let chain = budgets
            .get(id)
            .map(|b| b.chain(dining).to_vec())
            .unwrap_or_default();
        assert_eq!(chain.len(), 1);
        assert!(matches!(chain[0].limit, Limit::Onward { .. }));
        assert_eq!(
            budgets.cycle_rollover(id, world.category("Water"), today()),
            Err(BudgetError::NotBudgeted)
        );
    }

    #[test]
    fn the_5c_write_is_onward_on_change_stop_on_clear_and_nothing_when_unchanged() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        let dining = world.category("Dining");
        let write = |budgets: &mut Budgets, amount: Option<&str>| {
            budgets.set_monthly_limit(id, &world.categories, dining, amount.map(money), today())
        };
        assert_eq!(write(&mut budgets, None), Ok(false));
        assert_eq!(write(&mut budgets, Some("250.00")), Ok(true));
        assert_eq!(write(&mut budgets, Some("250")), Ok(false));
        assert_eq!(write(&mut budgets, Some("300.00")), Ok(true));
        assert_eq!(
            budgets.monthly_limit(id, &world.categories, dining, today()),
            Some(money("300.00"))
        );
        assert_eq!(write(&mut budgets, None), Ok(true));
        assert_eq!(
            budgets.monthly_limit(id, &world.categories, dining, today()),
            None
        );
    }

    #[test]
    fn a_parents_monthly_limit_is_the_sum_of_its_budgeted_leaves() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        set(
            &mut budgets,
            &world,
            id,
            "Groceries",
            sep(),
            Span::Onward,
            "500.00",
            None,
        );
        set(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep(),
            Span::MonthOnly,
            "300.00",
            None,
        );
        let food = world.category("Food");
        assert_eq!(
            budgets.monthly_limit(id, &world.categories, food, today()),
            Some(money("800.00"))
        );
        assert_eq!(
            budgets.monthly_limit(id, &world.categories, world.category("Housing"), today()),
            None
        );
    }

    #[test]
    fn a_leaf_that_gains_a_child_is_stopped_from_the_current_month() {
        let mut world = World::new();
        let (mut budgets, id) = world.budgets();
        set(
            &mut budgets,
            &world,
            id,
            "Transport",
            sep(),
            Span::Onward,
            "600.00",
            None,
        );
        set(
            &mut budgets,
            &world,
            id,
            "Transport",
            sep().next(),
            Span::Onward,
            "650.00",
            None,
        );
        let transport = world.category("Transport");
        let child = categories::insert_category(
            &mut world.categories,
            "Fuel".to_string(),
            Some(transport),
            CategoryTypes::Expense,
        );
        assert!(child.is_ok());
        assert_eq!(
            budgets.stop_new_parents(&world.categories, today()),
            vec![transport]
        );
        let chain = budgets
            .get(id)
            .map(|b| b.chain(transport).to_vec())
            .unwrap_or_default();
        assert_eq!(amount_in(&chain, sep()), None);
        assert_eq!(amount_in(&chain, sep().next()), None);
        // A second pass finds nothing left to stop.
        assert!(
            budgets
                .stop_new_parents(&world.categories, today())
                .is_empty()
        );
    }

    #[test]
    fn a_deleted_categorys_records_go_with_it_in_every_budget() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        set(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep(),
            Span::Onward,
            "250.00",
            None,
        );
        let dining = world.category("Dining");
        assert_eq!(budgets.limit_count(dining), 1);
        budgets.remove_category(dining);
        assert_eq!(budgets.limit_count(dining), 0);
    }

    // -- containers ----------------------------------------------------------------------------------------

    fn new_budget(world: &World, name: &str, start_from: StartFrom) -> NewBudget {
        NewBudget {
            name: name.to_string(),
            unit: "aud".to_string(),
            account_ids: vec![world.account("ANZ Everyday")],
            start_from,
        }
    }

    #[test]
    fn the_first_budget_is_the_default_and_later_ones_are_not() {
        let world = World::new();
        let (mut budgets, first) = world.budgets();
        let second = budgets.create(
            &new_budget(&world, "Second", StartFrom::Empty),
            &world.ledger(),
            today(),
        );
        assert_eq!(budgets.default_budget().map(|b| b.id), Some(first));
        assert_eq!(
            second
                .ok()
                .and_then(|id| budgets.get(id))
                .map(|b| b.is_default),
            Some(false)
        );
    }

    #[test]
    fn a_budget_needs_a_unique_name_and_same_unit_accounts_that_take_transactions() {
        let world = World::new();
        let (mut budgets, _) = world.budgets();
        let ledger = world.ledger();
        let create = |budgets: &mut Budgets, new: NewBudget| budgets.create(&new, &ledger, today());
        assert_eq!(
            create(&mut budgets, new_budget(&world, "  ", StartFrom::Empty)),
            Err(BudgetError::NameRequired)
        );
        assert_eq!(
            create(&mut budgets, new_budget(&world, "test", StartFrom::Empty)),
            Err(BudgetError::NameTaken)
        );
        let mut wrong_unit = new_budget(&world, "Crypto", StartFrom::Empty);
        wrong_unit.account_ids = vec![world.account("Bitcoin")];
        assert_eq!(
            create(&mut budgets, wrong_unit),
            Err(BudgetError::AccountNotInUnit)
        );
        let mut loan = new_budget(&world, "Loans", StartFrom::Empty);
        loan.account_ids = vec![world.account("Home Loan")];
        assert_eq!(
            create(&mut budgets, loan),
            Err(BudgetError::AccountNotInUnit)
        );
        let mut missing = new_budget(&world, "Missing", StartFrom::Empty);
        missing.account_ids = vec![9_999];
        assert_eq!(
            create(&mut budgets, missing),
            Err(BudgetError::AccountNotFound)
        );
    }

    #[test]
    fn start_from_copy_takes_the_sources_current_amounts_and_rollover_as_onward() {
        let world = World::new();
        let (mut budgets, source) = world.budgets();
        set(
            &mut budgets,
            &world,
            source,
            "Dining",
            sep(),
            Span::MonthOnly,
            "250.00",
            Some(Rollover::CarryBoth),
        );
        set(
            &mut budgets,
            &world,
            source,
            "Water",
            sep().next(),
            Span::Onward,
            "60.00",
            None,
        );
        let copy = budgets.create(
            &new_budget(&world, "Copy", StartFrom::Copy(source)),
            &world.ledger(),
            today(),
        );
        let copied = copy
            .ok()
            .and_then(|id| budgets.get(id))
            .cloned()
            .unwrap_or_else(unreachable_budget);
        let dining = copied.chain(world.category("Dining"));
        assert_eq!(amount_in(dining, sep()), Some(money("250.00")));
        assert_eq!(amount_in(dining, sep().next()), Some(money("250.00")));
        assert_eq!(
            applied(dining, sep()).map(|a| a.rollover),
            Some(Rollover::CarryBoth)
        );
        // A future-only amount isn't in the source's current month.
        assert!(copied.chain(world.category("Water")).is_empty());
    }

    #[test]
    fn start_from_last_three_months_leaves_a_zero_average_unbudgeted() {
        let mut world = World::new();
        world.post("ANZ Everyday", "Dining", date(2026, 8, 4), "-303.00");
        world.post("ANZ Everyday", "Dining", date(2026, 7, 4), "-300.00");
        let (mut budgets, _) = world.budgets();
        let created = budgets.create(
            &new_budget(&world, "Averages", StartFrom::LastThreeMonths),
            &world.ledger(),
            today(),
        );
        let made = created
            .ok()
            .and_then(|id| budgets.get(id))
            .cloned()
            .unwrap_or_else(unreachable_budget);
        // 603 over 3 months is 201, rounded to 200.
        assert_eq!(
            amount_in(made.chain(world.category("Dining")), sep()),
            Some(money("200.00"))
        );
        assert!(made.chain(world.category("Water")).is_empty());
        assert_eq!(
            applied(made.chain(world.category("Dining")), sep()).map(|a| a.rollover),
            Some(Rollover::None)
        );
    }

    #[test]
    fn edit_renames_and_changes_accounts_but_keeps_the_unit() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        let accounts = vec![world.account("Wallet")];
        assert_eq!(
            budgets.edit(id, " Renamed ", accounts.clone(), &world.accounts),
            Ok(())
        );
        let edited = budgets.get(id);
        assert_eq!(edited.map(|b| b.name.as_str()), Some("Renamed"));
        assert_eq!(edited.map(|b| b.account_ids.clone()), Some(accounts));
        assert_eq!(
            budgets.edit(id, "", vec![], &world.accounts),
            Err(BudgetError::NameRequired)
        );
        assert_eq!(
            budgets.edit(id, "X", vec![world.account("Bitcoin")], &world.accounts),
            Err(BudgetError::AccountNotInUnit)
        );
    }

    #[test]
    fn duplicating_copies_the_chains_and_numbers_a_taken_name() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        set(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep(),
            Span::Onward,
            "250.00",
            Some(Rollover::CarryUnspent),
        );
        let first = budgets.duplicate(id).unwrap_or_default();
        let second = budgets.duplicate(id).unwrap_or_default();
        assert_eq!(
            budgets.get(first).map(|b| b.name.as_str()),
            Some("Test copy")
        );
        assert_eq!(
            budgets.get(second).map(|b| b.name.as_str()),
            Some("Test copy 2")
        );
        let copy = budgets
            .get(first)
            .cloned()
            .unwrap_or_else(unreachable_budget);
        assert!(!copy.is_default && !copy.is_archived());
        assert_eq!(copy.unit, "aud");
        assert_eq!(
            copy.chain(world.category("Dining")),
            budgets
                .get(id)
                .map(|b| b.chain(world.category("Dining")))
                .unwrap_or_default()
        );
        assert_eq!(budgets.duplicate(99), Err(BudgetError::NotFound));
    }

    #[test]
    fn the_default_cant_be_archived_but_another_can_be_archived_and_restored() {
        let world = World::new();
        let (mut budgets, first) = world.budgets();
        let second = budgets
            .create(
                &new_budget(&world, "Second", StartFrom::Empty),
                &world.ledger(),
                today(),
            )
            .unwrap_or_default();
        assert_eq!(
            budgets.archive(first, today()),
            Err(BudgetError::DefaultCannotBeArchived)
        );
        assert_eq!(budgets.archive(second, today()), Ok(()));
        assert_eq!(
            budgets.get(second).and_then(|b| b.archived_at),
            Some(today())
        );
        assert_eq!(budgets.active().count(), 1);
        assert_eq!(budgets.archived().count(), 1);
        assert_eq!(budgets.restore(second), Ok(()));
        assert_eq!(budgets.active().count(), 2);
    }

    #[test]
    fn set_default_moves_the_flag_and_refuses_an_archived_budget() {
        let world = World::new();
        let (mut budgets, first) = world.budgets();
        let second = budgets
            .create(
                &new_budget(&world, "Second", StartFrom::Empty),
                &world.ledger(),
                today(),
            )
            .unwrap_or_default();
        let third = budgets
            .create(
                &new_budget(&world, "Third", StartFrom::Empty),
                &world.ledger(),
                today(),
            )
            .unwrap_or_default();
        assert_eq!(budgets.archive(third, today()), Ok(()));
        assert_eq!(budgets.set_default(third), Err(BudgetError::Archived));
        assert_eq!(budgets.set_default(second), Ok(()));
        assert_eq!(budgets.all().iter().filter(|b| b.is_default).count(), 1);
        assert_eq!(budgets.default_budget().map(|b| b.id), Some(second));
        // The old default can now be archived.
        assert_eq!(budgets.archive(first, today()), Ok(()));
    }

    #[test]
    fn an_archived_budget_is_read_only_but_still_computed() {
        let mut world = World::new();
        world.post("ANZ Everyday", "Dining", date(2026, 9, 4), "-40.00");
        let (mut budgets, first) = world.budgets();
        set(
            &mut budgets,
            &world,
            first,
            "Dining",
            sep(),
            Span::Onward,
            "250.00",
            None,
        );
        let second = budgets
            .create(
                &new_budget(&world, "Second", StartFrom::Empty),
                &world.ledger(),
                today(),
            )
            .unwrap_or_default();
        set(
            &mut budgets,
            &world,
            second,
            "Dining",
            sep(),
            Span::Onward,
            "100.00",
            None,
        );
        assert_eq!(budgets.archive(second, today()), Ok(()));
        let refused = budgets.set_amount(
            second,
            &world.categories,
            world.category("Dining"),
            sep(),
            Span::Onward,
            money("1.00"),
            None,
            today(),
        );
        assert_eq!(refused, Err(BudgetError::Archived));
        assert_eq!(
            budgets.edit(second, "New name", vec![], &world.accounts),
            Err(BudgetError::Archived)
        );
        let archived = budgets
            .get(second)
            .cloned()
            .unwrap_or_else(unreachable_budget);
        let figures = period_figures(&archived, &world.ledger(), sep(), today());
        assert_eq!(figures.budgeted, money("100.00"));
    }

    #[test]
    fn overlapping_budgets_count_the_same_split_independently() {
        let mut world = World::new();
        world.post("ANZ Everyday", "Groceries", date(2026, 9, 4), "-80.00");
        let (mut budgets, first) = world.budgets();
        let second = budgets
            .create(
                &new_budget(&world, "Second", StartFrom::Empty),
                &world.ledger(),
                today(),
            )
            .unwrap_or_default();
        set(
            &mut budgets,
            &world,
            first,
            "Groceries",
            sep(),
            Span::Onward,
            "500.00",
            None,
        );
        set(
            &mut budgets,
            &world,
            second,
            "Groceries",
            sep(),
            Span::Onward,
            "200.00",
            None,
        );
        let ledger = world.ledger();
        for (id, left) in [(first, "420.00"), (second, "120.00")] {
            let budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
            assert_eq!(
                period_figures(&budget, &ledger, sep(), today()).left,
                money(left)
            );
        }
    }

    // -- surface state and seed -------------------------------------------------------------------------------

    #[test]
    fn tabs_cycle_and_the_digits_pick_them() {
        assert_eq!(BudgetsTab::default(), BudgetsTab::Progress);
        assert_eq!(BudgetsTab::Progress.next(), BudgetsTab::Plan);
        assert_eq!(BudgetsTab::History.next(), BudgetsTab::Progress);
        assert_eq!(BudgetsTab::from_digit('3'), Some(BudgetsTab::History));
        assert_eq!(BudgetsTab::from_digit('4'), None);
    }

    fn seeded() -> (World, Budgets) {
        let mut world = World::new();
        world.transactions = default_transactions(
            &world.accounts,
            &world.categories,
            &default_payees(),
            &default_tags(),
            today(),
        );
        let budgets = default_budgets(&world.accounts, &world.categories, today());
        (world, budgets)
    }

    #[test]
    fn the_seed_has_a_default_personal_spending_and_an_overlapping_second_budget() {
        let (world, budgets) = seeded();
        assert_eq!(budgets.all().len(), 2);
        let personal = budgets.get(PERSONAL_SPENDING_ID);
        assert_eq!(personal.map(|b| b.name.as_str()), Some("Personal spending"));
        assert_eq!(
            budgets.default_budget().map(|b| b.id),
            Some(PERSONAL_SPENDING_ID)
        );
        assert_eq!(budgets.all().iter().filter(|b| b.is_default).count(), 1);
        let joint = budgets.get(2);
        assert!(joint.is_some_and(|j| {
            j.account_ids
                .iter()
                .all(|a| personal.is_some_and(|p| p.account_ids.contains(a)))
                && j.category_ids()
                    .any(|c| personal.is_some_and(|p| !p.chain(c).is_empty()))
        }));
        assert!(world.category("Groceries") > 0);
    }

    #[test]
    fn the_seed_only_budgets_valid_leaves_and_accounts_in_the_unit() {
        let (world, budgets) = seeded();
        for budget in budgets.all() {
            for id in budget.category_ids() {
                assert_eq!(check_leaf(&world.categories, id), Ok(()));
            }
            assert_eq!(
                check_accounts(&world.accounts, &budget.unit, &budget.account_ids),
                Ok(())
            );
            assert!(!budget.account_ids.is_empty());
        }
    }

    #[test]
    fn the_seed_produces_figures_with_parents_as_rollups() {
        let (world, budgets) = seeded();
        let personal = budgets
            .get(PERSONAL_SPENDING_ID)
            .cloned()
            .unwrap_or_else(unreachable_budget);
        let figures = period_figures(&personal, &world.ledger(), sep(), today());
        assert!(figures.budgeted.0 > zero());
        let household = row(&figures, world.category("Household"));
        assert!(household.is_some_and(|r| r.budget.is_some() && !r.is_parent));
        assert!(row(&figures, world.category("Food")).is_some_and(|r| r.is_parent));
    }

    #[test]
    fn dining_steps_up_next_month_in_the_seed() {
        let (world, budgets) = seeded();
        let chain = budgets
            .get(PERSONAL_SPENDING_ID)
            .map(|b| b.chain(world.category("Dining")).to_vec())
            .unwrap_or_default();
        assert_eq!(amount_in(&chain, sep()), Some(money("250.00")));
        assert_eq!(amount_in(&chain, sep().next()), Some(money("300.00")));
    }

    #[test]
    fn cents_money_is_exact() {
        assert_eq!(cents_money(-1850), money("-18.50"));
    }

    #[test]
    fn is_over_budget_reads_a_negative_spend_against_the_amount() {
        assert!(is_over_budget(&money("-318.40"), &money("250.00")));
        assert!(!is_over_budget(&money("-100.00"), &money("250.00")));
    }

    // -- category detail (9d) -------------------------------------------------------------------

    #[test]
    fn category_detail_lists_splits_largest_first_and_reads_the_track_record() {
        let mut world = World::new();
        let (budgets, id) = world.budgets();
        let dining = world.category("Dining");
        let Some(mut budget) = budgets.get(id).cloned() else {
            panic!("the Budget was created");
        };
        budget.limits.insert(
            dining,
            vec![LimitRecord {
                month: Period::of(date(2026, 6, 1)),
                limit: Limit::Onward {
                    amount: money("250.00"),
                    rollover: Rollover::None,
                },
            }],
        );
        // Over in August, under in July.
        world.post("ANZ Everyday", "Dining", date(2026, 8, 5), "-300.00");
        world.post("ANZ Everyday", "Dining", date(2026, 7, 5), "-100.00");
        world.post("ANZ Everyday", "Dining", date(2026, 9, 2), "-40.00");
        world.post("ANZ Everyday", "Dining", date(2026, 9, 9), "-278.40");
        // Off-budget Account: never counted.
        world.post("Amex Platinum", "Dining", date(2026, 9, 9), "-999.00");
        let detail = category_detail(&budget, &world.ledger(), dining, sep(), today())
            .expect("Dining is an Expense Category");

        assert_eq!(detail.figures.spent, money("318.40"));
        assert!(detail.figures.over);
        let amounts: Vec<Money> = detail.lines.iter().map(|l| l.amount.clone()).collect();
        assert_eq!(amounts, vec![money("278.40"), money("40.00")]);
        let track = detail.track.expect("earlier budgeted months exist");
        assert_eq!((track.months, track.over_months), (3, 1));
        assert_eq!(track.average, money("133.33"));
        assert_eq!(detail.rollover, Some(Rollover::None));
    }

    #[test]
    fn category_detail_of_a_parent_rolls_up_and_has_no_rollover() {
        let mut world = World::new();
        let (mut budgets, id) = world.budgets();
        set(
            &mut budgets,
            &world,
            id,
            "Groceries",
            sep(),
            Span::Onward,
            "500.00",
            None,
        );
        world.post("ANZ Everyday", "Groceries", date(2026, 9, 3), "-120.00");
        let Some(budget) = budgets.get(id) else {
            panic!("the Budget was created");
        };
        let parent = categories::path(&world.categories, world.category("Groceries"))
            .and_then(|_| {
                world
                    .categories
                    .iter()
                    .find(|c| {
                        !categories::is_leaf(&world.categories, c.id)
                            && categories::descendants_inclusive(&world.categories, c.id)
                                .contains(&world.category("Groceries"))
                    })
                    .map(|c| c.id)
            })
            .expect("Groceries sits under a parent");
        let detail = category_detail(budget, &world.ledger(), parent, sep(), today())
            .expect("a parent Expense Category");
        assert!(detail.figures.is_parent);
        assert_eq!(detail.figures.spent, money("120.00"));
        assert_eq!(detail.rollover, None);
    }

    #[test]
    fn category_detail_is_none_for_a_non_expense_category() {
        let world = World::new();
        let (budgets, id) = world.budgets();
        let Some(budget) = budgets.get(id) else {
            panic!("the Budget was created");
        };
        let income = world
            .categories
            .iter()
            .find(|c| c.category_type == CategoryTypes::Income)
            .map(|c| c.id)
            .unwrap_or_default();
        assert_eq!(
            category_detail(budget, &world.ledger(), income, sep(), today()),
            None
        );
    }

    // The Plan grid (9b)

    fn save(
        budgets: &mut Budgets,
        world: &World,
        id: u32,
        category: &str,
        month: Period,
        span: Span,
        text: &str,
    ) -> Result<bool, BudgetError> {
        budgets.save_cell(
            id,
            &world.categories,
            world.category(category),
            month,
            span,
            text,
            today(),
        )
    }

    fn chain_of(budgets: &Budgets, world: &World, id: u32, category: &str) -> Vec<LimitRecord> {
        budgets
            .get(id)
            .map(|budget| budget.chain(world.category(category)).to_vec())
            .unwrap_or_default()
    }

    #[test]
    fn saving_a_cell_onward_runs_until_the_next_record() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        assert_eq!(
            save(
                &mut budgets,
                &world,
                id,
                "Dining",
                sep(),
                Span::Onward,
                "250"
            ),
            Ok(true)
        );
        assert_eq!(
            save(
                &mut budgets,
                &world,
                id,
                "Dining",
                sep().next(),
                Span::Onward,
                "300"
            ),
            Ok(true)
        );
        let chain = chain_of(&budgets, &world, id, "Dining");
        assert_eq!(amount_in(&chain, sep()), Some(money("250.00")));
        assert_eq!(
            amount_in(&chain, sep().next().next()),
            Some(money("300.00"))
        );
    }

    #[test]
    fn saving_a_cell_month_only_leaves_the_next_month_on_the_onward_value() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        save(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep(),
            Span::Onward,
            "250",
        )
        .ok();
        save(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep().next(),
            Span::MonthOnly,
            "400",
        )
        .ok();
        let chain = chain_of(&budgets, &world, id, "Dining");
        assert_eq!(amount_in(&chain, sep().next()), Some(money("400.00")));
        assert_eq!(
            amount_in(&chain, sep().next().next()),
            Some(money("250.00"))
        );
    }

    #[test]
    fn typing_zero_writes_a_real_zero_and_not_unbudgeted() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        save(&mut budgets, &world, id, "Dining", sep(), Span::Onward, "0").ok();
        let chain = chain_of(&budgets, &world, id, "Dining");
        assert_eq!(amount_in(&chain, sep()), Some(money("0.00")));
    }

    #[test]
    fn clearing_a_cell_onward_writes_a_stop_from_that_month() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        save(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep(),
            Span::Onward,
            "250",
        )
        .ok();
        assert_eq!(
            save(
                &mut budgets,
                &world,
                id,
                "Dining",
                sep().next(),
                Span::Onward,
                ""
            ),
            Ok(true)
        );
        let chain = chain_of(&budgets, &world, id, "Dining");
        assert_eq!(amount_in(&chain, sep()), Some(money("250.00")));
        assert_eq!(amount_in(&chain, sep().next()), None);
        assert_eq!(amount_in(&chain, sep().next().next()), None);
    }

    #[test]
    fn clearing_a_cell_month_only_removes_that_months_own_month_only_record() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        save(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep(),
            Span::Onward,
            "250",
        )
        .ok();
        save(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep().next(),
            Span::MonthOnly,
            "400",
        )
        .ok();
        assert_eq!(
            save(
                &mut budgets,
                &world,
                id,
                "Dining",
                sep().next(),
                Span::MonthOnly,
                ""
            ),
            Ok(true)
        );
        let chain = chain_of(&budgets, &world, id, "Dining");
        assert_eq!(amount_in(&chain, sep().next()), Some(money("250.00")));
        // With no Month-only record there, a second clear has nothing to remove.
        assert_eq!(
            save(
                &mut budgets,
                &world,
                id,
                "Dining",
                sep().next(),
                Span::MonthOnly,
                ""
            ),
            Ok(false)
        );
    }

    #[test]
    fn saving_an_unchanged_amount_onward_writes_nothing() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        save(
            &mut budgets,
            &world,
            id,
            "Dining",
            sep(),
            Span::Onward,
            "250",
        )
        .ok();
        let before = chain_of(&budgets, &world, id, "Dining");
        assert_eq!(
            save(
                &mut budgets,
                &world,
                id,
                "Dining",
                sep().next(),
                Span::Onward,
                "250.00"
            ),
            Ok(false)
        );
        assert_eq!(chain_of(&budgets, &world, id, "Dining"), before);
    }

    #[test]
    fn a_closed_month_or_a_bad_amount_is_refused() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        assert_eq!(
            save(
                &mut budgets,
                &world,
                id,
                "Dining",
                sep().prev(),
                Span::Onward,
                "250"
            ),
            Err(BudgetError::ClosedMonth)
        );
        assert_eq!(
            save(
                &mut budgets,
                &world,
                id,
                "Dining",
                sep(),
                Span::Onward,
                "12x"
            ),
            Err(BudgetError::InvalidAmount)
        );
    }

    #[test]
    fn the_first_key_replaces_the_prefilled_amount_and_later_keys_edit_it() {
        let mut edit = PlanEdit::new(1, sep(), Some(&money("250.00")));
        assert_eq!(edit.text, "250.00");
        edit.type_char('3');
        edit.type_char('x');
        edit.type_char('0');
        edit.type_char('.');
        edit.type_char('.');
        edit.type_char('5');
        assert_eq!(edit.text, "30.5");
        edit.backspace();
        assert_eq!(edit.text, "30.");
    }

    #[test]
    fn backspace_on_a_prefilled_cell_clears_it_at_once() {
        let mut edit = PlanEdit::new(1, sep(), Some(&money("250.00")));
        edit.backspace();
        assert_eq!(edit.text, "");
    }

    #[test]
    fn the_plan_groups_leaves_under_their_parent_and_totals_each_month() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        save(
            &mut budgets,
            &world,
            id,
            "Rent",
            sep(),
            Span::Onward,
            "1000",
        )
        .ok();
        save(
            &mut budgets,
            &world,
            id,
            "Electricity",
            sep(),
            Span::Onward,
            "90",
        )
        .ok();
        save(&mut budgets, &world, id, "Water", sep(), Span::Onward, "60").ok();
        save(
            &mut budgets,
            &world,
            id,
            "Transport",
            sep().next(),
            Span::Onward,
            "40",
        )
        .ok();
        let Some(budget) = budgets.get(id) else {
            panic!("the Budget was created");
        };
        let start = default_plan_start(today());
        let grid = plan(budget, &world.ledger(), start, today());
        assert_eq!(grid.months.len(), PLAN_MONTHS);
        assert_eq!(grid.months.get(2), Some(&sep()));
        let labels: Vec<Option<&str>> = grid
            .sections
            .iter()
            .map(|section| section.parent.as_deref())
            .collect();
        // A top-level leaf (Transport) closes the grid as the unlabelled group.
        assert_eq!(labels, vec![Some("Housing"), Some("Utilities"), None]);
        assert_eq!(grid.row_count(), 4);
        // Sep is the third month; Oct the fourth.
        assert_eq!(grid.totals.get(2), Some(&money("1150.00")));
        assert_eq!(grid.totals.get(3), Some(&money("1190.00")));
        assert_eq!(grid.totals.first(), Some(&money("0")));
        let cell = grid
            .row(0)
            .and_then(|row| row.cells.get(2))
            .expect("Rent's September cell");
        assert!(cell.own_record && !cell.closed);
        assert!(
            grid.row(0)
                .is_some_and(|row| row.cells.first().is_some_and(|c| c.closed))
        );
        // Unallocated is average income less the month's total.
        assert_eq!(
            grid.unallocated.get(2).map(|left| left.0.clone()),
            Some(grid.average_income.0.clone() - money("1150.00").0)
        );
    }

    #[test]
    fn a_cleared_row_stays_in_the_grid_while_a_record_starts_in_the_range() {
        let world = World::new();
        let (mut budgets, id) = world.budgets();
        save(
            &mut budgets,
            &world,
            id,
            "Rent",
            sep().next(),
            Span::Onward,
            "1000",
        )
        .ok();
        save(
            &mut budgets,
            &world,
            id,
            "Rent",
            sep().next(),
            Span::Onward,
            "",
        )
        .ok();
        let Some(budget) = budgets.get(id) else {
            panic!("the Budget was created");
        };
        let grid = plan(
            budget,
            &world.ledger(),
            default_plan_start(today()),
            today(),
        );
        assert_eq!(grid.row_count(), 1);
        assert!(
            grid.row(0)
                .is_some_and(|row| row.cells.iter().all(|c| c.amount.is_none()))
        );
    }

    #[test]
    fn the_plan_range_is_bounded_by_the_first_amount_and_a_year_ahead() {
        let world = World::new();
        let (budgets, id) = world.budgets();
        let Some(budget) = budgets.get(id) else {
            panic!("the Budget was created");
        };
        let (earliest, latest) = plan_start_bounds(budget, today());
        assert_eq!(earliest, default_plan_start(today()));
        assert_eq!(latest, (0..12).fold(earliest, |month, _| month.next()));
    }
}
