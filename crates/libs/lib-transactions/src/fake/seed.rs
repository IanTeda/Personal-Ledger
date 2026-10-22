//! The stub Transactions generator (`docs/ux/desktop-mockups/03-transactions/README.md`): every
//! rule the Transactions table and filters must handle has a row to exercise it.
//!
//! It is deterministic (fixed seed, injectable `today`): per-account counts equal each
//! [`SeedAccount`]'s `transaction_count` for the accounts that take Transactions (Cash, Bank and
//! Credit Card; Loan and Investment accounts take Repayments and Trades instead), about 8% of the
//! random ones are multi-Split, and a few hand-authored cases always exist.
//!
//! The other domains' stubs reach it as names ([`SeedNames`]) and plain account rows rather than as
//! their crates, since the Categories, Payees and Tags crates depend on this one. `lib-ledger`
//! joins it to their stubs (`default_transactions`).
//!
//! No transaction account in this seed is in a second Unit, so the mixed-Units rule (the running
//! total blanks) is exercised by the filter engine's unit tests rather than visible in the app
//! until an account in another Unit holds transactions; seeding one would mean changing the
//! Accounts and Settings stubs the earlier maps reconciled to their mockups.

use bigdecimal::BigDecimal;
use chrono::{Duration, NaiveDate};
use lib_core::{
    AccountId, AccountType, CategoryId, Money, PayeeId, TagId, TransactionId, TransactionStatus,
};

use crate::{Split, Transaction, takes_transactions};

pub const HISTORY_DAYS: i64 = 600;

/// The seed for [`default_transactions`]; each account derives its own stream from it.
pub const DEFAULT_SEED: u64 = 0x5EED_1EDE_CAFE_0001;

/// The share of random transactions (in percent) that are multi-Split, before the hand-authored
/// ones are added.
const MULTI_SPLIT_PERCENT: u64 = 14;

/// Builds an exact `Money` from a signed count of cents: `-1850` is `-18.50`.
fn cents_money(cents: i64) -> Money {
    Money(BigDecimal::new(cents.into(), 2))
}

/// A small, dependency-free, deterministic generator (splitmix64).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `lo..=hi`.
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + (self.next() % (hi - lo + 1) as u64) as i64
    }

    fn chance(&mut self, percent: u64) -> bool {
        self.next() % 100 < percent
    }

    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[(self.next() % items.len() as u64) as usize]
    }
}

/// The other domains' stub rows the seed names, looked up by name so it reads in names rather
/// than ids. `None` for a name the stub doesn't have.
pub trait SeedNames {
    fn category(&self, name: &str) -> Option<CategoryId>;

    fn payee(&self, name: &str) -> Option<PayeeId>;

    fn tag(&self, name: &str) -> Option<TagId>;
}

/// One stub Account, as much of it as the seed reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedAccount {
    pub id: AccountId,
    pub name: String,
    pub account_type: AccountType,
    /// How many Transactions the seed gives it.
    pub transaction_count: u32,
}

struct Lookup<'a> {
    names: &'a dyn SeedNames,
}

impl Lookup<'_> {
    fn split(
        &self,
        cents: i64,
        category: &str,
        payee: Option<&str>,
        tag_names: &[&str],
    ) -> Option<Split> {
        Some(Split {
            amount: cents_money(cents),
            category_id: self.names.category(category)?,
            payee_id: match payee {
                Some(name) => Some(self.names.payee(name)?),
                None => None,
            },
            tag_ids: tag_names
                .iter()
                .filter_map(|name| self.names.tag(name))
                .collect(),
        })
    }
}

/// What a random transaction in one Category looks like.
struct Template {
    category: &'static str,
    payees: &'static [&'static str],
    /// Magnitude range in cents, before the account's scale.
    min: i64,
    max: i64,
    income: bool,
    /// The Category a multi-Split version of this transaction also spends in.
    companion: Option<&'static str>,
    memos: &'static [&'static str],
}

const TEMPLATES: &[Template] = &[
    Template {
        category: "Groceries",
        payees: &["Woolworths", "Coles", "Aldi Kelvin Grove"],
        min: 1_200,
        max: 24_000,
        income: false,
        companion: Some("Household"),
        memos: &["weekly shop", "top-up", "party supplies"],
    },
    Template {
        category: "Dining",
        payees: &["Kura Sushi", "Cafe Vittoria"],
        min: 800,
        max: 9_000,
        income: false,
        companion: Some("Groceries"),
        memos: &["lunch", "dinner out", "coffee"],
    },
    Template {
        category: "Transport",
        payees: &["BP", "Uber"],
        min: 1_500,
        max: 9_500,
        income: false,
        companion: None,
        memos: &["fuel", "ride home"],
    },
    Template {
        category: "Household",
        payees: &["Bunnings Warehouse", "Kmart", "Officeworks"],
        min: 1_500,
        max: 30_000,
        income: false,
        companion: Some("Groceries"),
        memos: &["garden", "supplies"],
    },
    Template {
        category: "Electricity",
        payees: &["Origin Energy"],
        min: 8_000,
        max: 32_000,
        income: false,
        companion: None,
        memos: &["quarterly bill"],
    },
    Template {
        category: "Water",
        payees: &["Sydney Water"],
        min: 3_000,
        max: 12_000,
        income: false,
        companion: None,
        memos: &["quarterly bill"],
    },
    Template {
        category: "Salary",
        payees: &["Employer Pty Ltd"],
        min: 380_000,
        max: 440_000,
        income: true,
        companion: None,
        memos: &["pay"],
    },
    Template {
        category: "Interest",
        payees: &["ANZ Banking Group"],
        min: 500,
        max: 25_000,
        income: true,
        companion: None,
        memos: &["monthly interest"],
    },
];

/// Which templates (with a weight) an account draws from, and the percentage its amounts are
/// scaled by -- so Wallet's amounts are small and the Offset account sees mostly interest and
/// bills. `None` for an account the seed doesn't populate.
fn profile(account_name: &str) -> Option<(&'static [(&'static str, u32)], i64)> {
    match account_name {
        "Wallet" => Some((&[("Groceries", 3), ("Dining", 3), ("Transport", 2)], 30)),
        "ANZ Everyday" => Some((
            &[
                ("Groceries", 4),
                ("Dining", 3),
                ("Transport", 2),
                ("Household", 2),
                ("Electricity", 1),
                ("Water", 1),
                ("Salary", 1),
            ],
            100,
        )),
        "ANZ Offset" => Some((
            &[
                ("Interest", 3),
                ("Household", 1),
                ("Electricity", 1),
                ("Water", 1),
            ],
            100,
        )),
        "Amex Platinum" => Some((
            &[
                ("Dining", 3),
                ("Household", 3),
                ("Groceries", 2),
                ("Transport", 1),
            ],
            100,
        )),
        _ => None,
    }
}

/// The Tags random transactions draw from, weighted so the Tags page keeps the handoff's usage
/// order (shared, reimbursable, tax-deductible, work-trip, gift).
const RANDOM_TAGS: [(&str, u32); 5] = [
    ("shared", 38),
    ("reimbursable", 24),
    ("tax-deductible", 12),
    ("work-trip", 14),
    ("gift", 6),
];

/// The share of random transactions (in percent) with one tagged Split.
const TAGGED_PERCENT: u64 = 28;

/// How recently a transaction happened decides how confirmed it is: old ones are mostly
/// reconciled, recent ones mostly open.
fn status_for(rng: &mut Rng, age_days: i64) -> TransactionStatus {
    let roll = rng.range(0, 99);
    match age_days {
        ..=7 if roll < 80 => TransactionStatus::Open,
        ..=7 => TransactionStatus::Cleared,
        8..=30 if roll < 30 => TransactionStatus::Open,
        8..=30 => TransactionStatus::Cleared,
        31..=90 if roll < 60 => TransactionStatus::Reconciled,
        31..=90 => TransactionStatus::Cleared,
        _ if roll < 90 => TransactionStatus::Reconciled,
        _ => TransactionStatus::Cleared,
    }
}

fn weighted<'a>(rng: &mut Rng, weights: &'a [(&'static str, u32)]) -> &'a str {
    let total: u32 = weights.iter().map(|(_, weight)| weight).sum();
    let mut roll = (rng.next() % u64::from(total)) as u32;
    for (name, weight) in weights {
        if roll < *weight {
            return name;
        }
        roll -= weight;
    }
    weights[0].0
}

fn random_transaction(
    rng: &mut Rng,
    lookup: &Lookup<'_>,
    template: &Template,
    scale_percent: i64,
    account_id: AccountId,
    today: NaiveDate,
) -> Option<Transaction> {
    let age = rng.range(0, HISTORY_DAYS - 1);
    let date = today - Duration::days(age);
    let magnitude = (rng.range(template.min, template.max) * scale_percent / 100).max(100);
    let sign = if template.income { 1 } else { -1 };

    let payee_name = *rng.pick(template.payees);
    let mut parts: Vec<(i64, &str, &str)> = Vec::new();

    let companion = template
        .companion
        .filter(|_| !template.income && magnitude >= 600 && rng.chance(MULTI_SPLIT_PERCENT));
    match companion {
        Some(companion_name) => {
            let second = (magnitude * rng.range(15, 40) / 100).max(100);
            let third = if rng.chance(25) {
                (magnitude * rng.range(5, 15) / 100).max(100)
            } else {
                0
            };
            let primary = magnitude - second - third;
            parts.push((primary, template.category, payee_name));
            let companion_payee = if rng.chance(70) {
                payee_name
            } else {
                let companion_template = TEMPLATES
                    .iter()
                    .find(|candidate| candidate.category == companion_name)?;
                *rng.pick(companion_template.payees)
            };
            parts.push((second, companion_name, companion_payee));
            if third > 0 {
                let third_name = if companion_name == "Household" {
                    "Groceries"
                } else {
                    "Household"
                };
                parts.push((third, third_name, payee_name));
            }
        }
        None => parts.push((magnitude, template.category, payee_name)),
    }

    let tagged_index = rng
        .chance(TAGGED_PERCENT)
        .then(|| rng.range(0, parts.len() as i64 - 1) as usize);
    let tag_name = weighted(rng, &RANDOM_TAGS);
    let mut splits = Vec::with_capacity(parts.len());
    for (index, (cents, category, payee)) in parts.into_iter().enumerate() {
        let tags: &[&str] = if tagged_index == Some(index) {
            std::slice::from_ref(&tag_name)
        } else {
            &[]
        };
        splits.push(lookup.split(sign * cents, category, Some(payee), tags)?);
    }

    let description = (!template.memos.is_empty() && rng.chance(55))
        .then(|| (*rng.pick(template.memos)).to_string());

    Some(Transaction {
        id: TransactionId::seed(0),
        date,
        account_id,
        status: status_for(rng, age),
        is_flagged: rng.chance(3),
        description,
        splits,
    })
}

/// The hand-authored cases every seed contains, as `(account name, transaction)`. Each exists to
/// exercise a rule: a groceries plus household receipt from one Payee, a receipt whose Splits carry
/// different Payees, a receipt where only one Split has a Tag, a salary (positive, income
/// Category), one flagged transaction in each status, and the Tags page's hand-placed usage: three
/// `one-off`s, the `Work Trip` likely duplicate and the inactive `Bali 2025`.
fn specials(lookup: &Lookup<'_>, today: NaiveDate) -> Vec<(&'static str, Transaction)> {
    let day = |days: i64| today - Duration::days(days);
    let make = |account: &'static str,
                days: i64,
                status: TransactionStatus,
                is_flagged: bool,
                description: Option<&str>,
                splits: Vec<Option<Split>>|
     -> Option<(&'static str, Transaction)> {
        Some((
            account,
            Transaction {
                id: TransactionId::seed(0),
                date: day(days),
                account_id: AccountId::seed(0),
                status,
                is_flagged,
                description: description.map(str::to_string),
                splits: splits.into_iter().collect::<Option<Vec<_>>>()?,
            },
        ))
    };
    use TransactionStatus::{Cleared, Open, Reconciled};

    [
        make(
            "ANZ Everyday",
            2,
            Cleared,
            false,
            Some("Weekly shop and cleaning supplies"),
            vec![
                lookup.split(-18_000, "Groceries", Some("Woolworths"), &[]),
                lookup.split(-3_430, "Household", Some("Woolworths"), &[]),
            ],
        ),
        make(
            "Amex Platinum",
            4,
            Open,
            false,
            Some("Airport"),
            vec![
                lookup.split(-1_850, "Dining", Some("Hudson News"), &[]),
                lookup.split(-3_200, "Household", Some("Kmart"), &[]),
            ],
        ),
        make(
            "ANZ Everyday",
            6,
            Cleared,
            false,
            Some("Tokyo dinner and souvenirs"),
            vec![
                lookup.split(-6_400, "Dining", Some("Kura Sushi"), &["Japan Trip 2026"]),
                lookup.split(-3_000, "Household", Some("Don Quijote"), &[]),
            ],
        ),
        make(
            "ANZ Everyday",
            9,
            Reconciled,
            false,
            None,
            vec![lookup.split(421_000, "Salary", Some("Employer Pty Ltd"), &[])],
        ),
        make(
            "ANZ Everyday",
            12,
            Open,
            true,
            Some("Bill looks high"),
            vec![lookup.split(-31_240, "Electricity", Some("Origin Energy"), &[])],
        ),
        make(
            "Amex Platinum",
            40,
            Cleared,
            true,
            Some("Check the receipt"),
            vec![lookup.split(-8_900, "Household", Some("Officeworks"), &[])],
        ),
        make(
            "ANZ Offset",
            100,
            Reconciled,
            true,
            Some("Interest looks off"),
            vec![lookup.split(21_235, "Interest", Some("ANZ Banking Group"), &[])],
        ),
        make(
            "ANZ Everyday",
            5,
            Open,
            false,
            Some("Team lunch"),
            vec![lookup.split(-8_500, "Dining", Some("Cafe Vittoria"), &[])],
        ),
        make(
            "Amex Platinum",
            7,
            Cleared,
            false,
            Some("Client dinner"),
            vec![
                lookup.split(-15_200, "Dining", Some("Kura Sushi"), &[]),
                lookup.split(-4_100, "Household", Some("Kura Sushi"), &[]),
            ],
        ),
        make(
            "ANZ Everyday",
            14,
            Open,
            false,
            Some("Date night and drinks"),
            vec![lookup.split(-12_300, "Dining", Some("Cafe Vittoria"), &[])],
        ),
    ]
    .into_iter()
    .chain(
        // J Smith: a Payee with no default Category, used by four transfers.
        [20, 50, 80, 110].map(|days| {
            make(
                "ANZ Everyday",
                days,
                Reconciled,
                false,
                Some("Shared bills"),
                vec![lookup.split(-16_000, "Household", Some("J Smith"), &[])],
            )
        }),
    )
    .chain([3, 45, 160].map(|days| {
        make(
            "Amex Platinum",
            days,
            Cleared,
            false,
            Some("One-off purchase"),
            vec![lookup.split(
                -6_500,
                "Household",
                Some("Bunnings Warehouse"),
                &["one-off"],
            )],
        )
    }))
    .chain([30, 31].map(|days| {
        make(
            "Amex Platinum",
            days,
            Cleared,
            false,
            Some("Conference travel"),
            vec![lookup.split(-4_800, "Transport", Some("Uber"), &["Work Trip"])],
        )
    }))
    .chain([500, 503].map(|days| {
        make(
            "ANZ Everyday",
            days,
            Reconciled,
            false,
            Some("Bali holiday"),
            vec![lookup.split(-9_000, "Dining", Some("Cafe Vittoria"), &["Bali 2025"])],
        )
    }))
    .flatten()
    .collect()
}

/// The stub Transactions: newest first, ids assigned oldest-first (a higher id is newer). Per-account
/// counts equal each account's `transaction_count`; accounts that take no Transactions, and any
/// account the seed has no profile for, get none.
pub fn stub_transactions(
    accounts: &[SeedAccount],
    names: &dyn SeedNames,
    today: NaiveDate,
) -> Vec<Transaction> {
    let lookup = Lookup { names };
    let specials = specials(&lookup, today);
    let mut all: Vec<Transaction> = Vec::new();

    for account in accounts
        .iter()
        .filter(|a| takes_transactions(&a.account_type))
    {
        let Some((weights, scale)) = profile(&account.name) else {
            continue;
        };
        let wanted = account.transaction_count as usize;
        let mut mine: Vec<Transaction> = specials
            .iter()
            .filter(|(name, _)| *name == account.name)
            .map(|(_, transaction)| Transaction {
                account_id: account.id,
                ..transaction.clone()
            })
            .take(wanted)
            .collect();

        let mut rng =
            Rng(DEFAULT_SEED
                ^ u64::from(account.id.seed_number()).wrapping_mul(0xA24B_AED4_963E_E407));
        while mine.len() < wanted {
            let name = weighted(&mut rng, weights);
            let Some(template) = TEMPLATES.iter().find(|t| t.category == name) else {
                break;
            };
            match random_transaction(&mut rng, &lookup, template, scale, account.id, today) {
                Some(transaction) => mine.push(transaction),
                // A category or payee the stubs don't have: nothing more can be generated.
                None => break,
            }
        }
        all.extend(mine);
    }

    all.sort_by_key(|transaction| (transaction.date, transaction.account_id));
    for (index, transaction) in all.iter_mut().enumerate() {
        transaction.id = TransactionId::seed(index as u32 + 1);
    }
    all.reverse();
    all
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cents_money_keeps_two_places_and_the_sign() {
        assert_eq!(cents_money(-1850).0.to_string(), "-18.50");
        assert_eq!(cents_money(421_000).0.to_string(), "4210.00");
        assert_eq!(cents_money(5).0.to_string(), "0.05");
    }

    #[test]
    fn the_generator_is_inclusive_at_both_ends_of_a_range() {
        let mut rng = Rng(1);
        let mut seen_lo = false;
        let mut seen_hi = false;
        for _ in 0..500 {
            match rng.range(3, 5) {
                3 => seen_lo = true,
                5 => seen_hi = true,
                4 => {}
                other => panic!("{other} is out of range"),
            }
        }
        assert!(seen_lo && seen_hi);
    }
}
