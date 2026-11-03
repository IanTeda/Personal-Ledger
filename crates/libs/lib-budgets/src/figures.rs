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

/// Whether spending (an expense amount, negative) exceeds the budget for this category.
/// Categories 5c's row bar reads it against the month-to-date sum, which keeps the ledger's sign.
pub fn is_over_budget(spent: &Money, budget: &Money) -> bool {
    spent.0 < budget.0.clone() * -1 // spent is negative; over-budget when |spent| > budget
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;

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
            world.ledger(),
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
        let found = effective_budget(&budget, world.ledger(), electricity, sep(), today());
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
        let found = effective_budget(&budget, world.ledger(), electricity, sep(), today());
        assert_eq!(found.map(|e| e.carried_in), Some(money("60.00")));
    }

    #[test]
    fn spent_is_signed_and_counts_only_on_budget_accounts() {
        let (world, budgets, id) = september();
        let figures = period_figures(
            &budgets.get(id).cloned().unwrap_or_else(unreachable_budget),
            world.ledger(),
            sep(),
            today(),
        );
        let groceries = row(&figures, world.category("Groceries"));
        // 120 spent, 20 refunded, the Amex purchase off budget.
        assert_eq!(groceries.map(|r| r.spent.clone()), Some(money("100.00")));
        let dining = row(&figures, world.category("Dining"));
        assert_eq!(dining.map(|r| r.spent.clone()), Some(money("318.40")));
    }

    /// A Budget keeps the id of an Account deleted since it named it, until Budgets persist with a
    /// cascading link (#811): that id counts nothing, so the Budget counts only its other Accounts.
    #[test]
    fn a_budget_naming_a_missing_account_counts_only_its_other_accounts() {
        let (mut world, budgets, id) = september();
        let offset = world.account("ANZ Offset");
        world.reads.accounts.retain(|account| account.id != offset);
        let budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
        assert!(
            budget.account_ids.contains(&offset),
            "the Budget keeps the id"
        );

        let figures = period_figures(&budget, world.ledger(), sep(), today());

        // Everyday's 200 alone: Offset's 118.40 isn't counted, though a row still names it.
        let dining = row(&figures, world.category("Dining"));
        assert_eq!(dining.map(|r| r.spent.clone()), Some(money("200.00")));
        assert!(!budget.counts(&world.reads.accounts, offset));
        assert!(budget.counts(&world.reads.accounts, world.account("ANZ Everyday")));
    }

    /// A Budget left with no Account compares nothing.
    #[test]
    fn a_budget_whose_accounts_are_all_missing_compares_nothing() {
        let (mut world, budgets, id) = september();
        let budget = budgets.get(id).cloned().unwrap_or_else(unreachable_budget);
        world
            .reads
            .accounts
            .retain(|account| !budget.account_ids.contains(&account.id));

        let figures = period_figures(&budget, world.ledger(), sep(), today());

        assert_eq!(figures.spent, money("0"));
    }

    #[test]
    fn a_leaf_over_its_budget_counts_and_carries_the_negative_left() {
        let (world, budgets, id) = september();
        let figures = period_figures(
            &budgets.get(id).cloned().unwrap_or_else(unreachable_budget),
            world.ledger(),
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
            world.ledger(),
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
            world.ledger(),
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
            world.ledger(),
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
            world.ledger(),
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
            world.ledger(),
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

        let now = period_figures(&budget, world.ledger(), sep(), today());
        assert_eq!(
            row(&now, electricity).map(|r| r.known.clone()),
            Some(money("105.00"))
        );
        assert_eq!(now.left, money("95.00"));
        // The four unpaid bills on the Budget's Account: two this month, one carried, none off-budget.
        assert_eq!(now.known_count, 3);

        let next = period_figures(&budget, world.ledger(), sep().next(), today());
        assert_eq!(
            row(&next, electricity).map(|r| r.known.clone()),
            Some(money("9.00"))
        );

        let past = period_figures(&budget, world.ledger(), sep().prev(), today());
        assert!(row(&past, electricity).is_none());
    }

    #[test]
    fn a_paid_bill_stops_counting_once_it_is_no_longer_open() {
        let mut world = World::new();
        world.bill("ANZ Everyday", "Electricity", date(2026, 9, 28), "80.00");
        // Skipped, so no longer open.
        world.reads.open_bills.clear();
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
            world.ledger(),
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
            world.ledger(),
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
            world.ledger(),
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
        let now = period_figures(&budget, world.ledger(), sep(), today());
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

        let past = period_figures(&budget, world.ledger(), sep().prev(), today());
        assert_eq!((past.elapsed.percent, past.elapsed.days_left), (100, 0));
        assert_eq!(past.per_day, None);

        let future = period_figures(&budget, world.ledger(), sep().next(), today());
        assert_eq!((future.elapsed.percent, future.elapsed.days_left), (0, 31));
    }

    #[test]
    fn health_summarises_the_current_month() {
        let (world, budgets, id) = september();
        let health = health(
            &budgets.get(id).cloned().unwrap_or_else(unreachable_budget),
            world.ledger(),
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
                world.ledger(),
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
        let figures = period_figures(&budget, world.ledger(), sep(), today());
        let bars = dashboard_bars(&figures, world.ledger());
        assert!(bars.len() <= DASHBOARD_BARS);
        for bar in &bars {
            let category = tree::get(&world.reads.categories, bar.category_id);
            assert_eq!(category.map(|c| c.parent), Some(None));
        }
        // Rent (90%) leads, as its top-level parent's rollup; Household is unbudgeted and so is
        // never shown.
        let top_of = |name: &str| {
            let id = world.category(name);
            world
                .reads
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

    #[test]
    fn overlapping_budgets_count_the_same_split_independently() {
        let mut world = World::new();
        world.post("ANZ Everyday", "Groceries", date(2026, 9, 4), "-80.00");
        let (mut budgets, first) = world.budgets();
        let second = budgets
            .create(
                &new_budget(&world, "Second", StartFrom::Empty),
                world.ledger(),
                today(),
            )
            .expect("the id exists");
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
                period_figures(&budget, ledger, sep(), today()).left,
                money(left)
            );
        }
    }

    #[test]
    fn is_over_budget_reads_a_negative_spend_against_the_amount() {
        assert!(is_over_budget(&money("-318.40"), &money("250.00")));
        assert!(!is_over_budget(&money("-100.00"), &money("250.00")));
    }
}
