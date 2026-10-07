//! The stub dataset: the seeded Budgets and their Category Limits.

use std::collections::BTreeMap;

use chrono::NaiveDate;

use crate::{
    accounts::Account,
    categories::{self, Category},
    period::Period,
    transactions,
};

use super::store::Budgets;
use super::{Budget, Limit, LimitRecord, Method, PERSONAL_SPENDING_ID, Rollover, cents_money, put};

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
