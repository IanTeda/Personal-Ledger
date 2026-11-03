//! The Budget: a named container with one Category Limit chain per leaf Expense Category, the
//! records a chain holds and what a month resolves to along one.

use std::collections::BTreeMap;

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use lib_core::{AccountId, BudgetId, CategoryId, Money, Period, UnitId};

use crate::BudgetAccount;

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
    pub id: BudgetId,
    pub name: String,
    pub method: Method,
    /// The Unit code (e.g. `"aud"`), locked at creation; every on-budget Account shares it.
    pub unit: UnitId,
    /// The Account ids whose Splits this Budget counts. Storage drops a deleted Account's id
    /// (its link cascades), but a Budget already in memory keeps it until it next loads, and
    /// ignores it: see [`Budget::counts`].
    pub account_ids: Vec<AccountId>,
    pub is_default: bool,
    /// Archived is computed and viewable but read-only; `None` while active.
    pub archived_at: Option<NaiveDate>,
    /// One chain per leaf Expense Category id, sorted by month with one record per month.
    ///
    /// Public only so the Desktop's seeded-data tests can build a chain directly; everything else
    /// goes through [`Budgets`] and keeps the chain sorted.
    pub limits: BTreeMap<CategoryId, Vec<LimitRecord>>,
}

impl Budget {
    /// Whether the Budget counts Account `id`: one of its [`Budget::account_ids`] that names one
    /// of `accounts`. An id with no Account counts nothing, so a Budget left with none compares
    /// nothing (#854).
    pub fn counts(&self, accounts: &[BudgetAccount], id: AccountId) -> bool {
        self.account_ids.contains(&id) && accounts.iter().any(|account| account.id == id)
    }
}

impl Budget {
    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }

    /// A Category's chain, oldest record first.
    pub fn chain(&self, category_id: CategoryId) -> &[LimitRecord] {
        self.limits.get(&category_id).map_or(&[], Vec::as_slice)
    }

    /// The Categories that have (or once had) a chain here.
    pub fn category_ids(&self) -> impl Iterator<Item = CategoryId> + '_ {
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

    /// The first month any record starts in, a Stop included: where a carry's walk begins.
    pub(crate) fn first_record_month(&self) -> Option<Period> {
        self.limits
            .values()
            .flatten()
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
pub fn inherited_rollover(chain: &[LimitRecord], month: Period) -> Rollover {
    match chain.iter().rfind(|record| record.month < month) {
        Some(LimitRecord {
            limit: Limit::Onward { rollover, .. } | Limit::MonthOnly { rollover, .. },
            ..
        }) => *rollover,
        _ => Rollover::None,
    }
}

pub fn put(chain: &mut Vec<LimitRecord>, record: LimitRecord) {
    chain.retain(|existing| existing.month != record.month);
    chain.push(record);
    chain.sort_by_key(|record| record.month);
}

pub fn zero() -> BigDecimal {
    BigDecimal::from(0)
}

/// Builds an exact `Money` from a signed count of cents: `-1850` is `-18.50`.
pub fn cents_money(cents: i64) -> Money {
    Money(BigDecimal::new(cents.into(), 2))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;

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

    #[test]
    fn cents_money_is_exact() {
        assert_eq!(cents_money(-1850), money("-18.50"));
    }
}
