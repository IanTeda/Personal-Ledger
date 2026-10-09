//! Pure Budgets domain types and the stub dataset behind them (`docs/ux/desktop-mockups/14-budgets-v2/`,
//! ADR-0028 and ADR-0029) -- `gpui`-free and in-memory, the same "pure state, chrome renders it"
//! split `bills/` uses. Nothing here reads `lib_database`.
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
//!
//! This file holds the Category Limit chain itself; each surface's figures sit in their own
//! section module, re-exported here. The Desktop reads it through its `BudgetsStore` Entity.

mod detail;
mod figures;
mod history;
mod plan;
mod seed;
mod store;

pub use detail::*;
pub use figures::*;
pub use history::*;
pub use plan::*;
pub use seed::*;
pub use store::*;

use std::collections::BTreeMap;

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use lib_core::{Money, Period};

/// The id the seeded **Personal spending** Budget carries: Categories 5c binds it by id, so a
/// rename changes nothing (ADR-0029).
pub const PERSONAL_SPENDING_ID: u32 = 1;

/// How far back "the last 3 months" reaches, for the average and Unallocated figures.
pub const AVERAGE_MONTHS: i32 = 3;

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
    ///
    /// Public only so the Desktop's seeded-data tests can build a chain directly; everything else
    /// goes through [`Budgets`] and keeps the chain sorted.
    pub limits: BTreeMap<u32, Vec<LimitRecord>>,
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
