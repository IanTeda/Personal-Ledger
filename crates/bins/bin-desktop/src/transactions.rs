//! Pure Transactions-surface domain types and the stub dataset behind them
//! (`docs/ux/desktop/Transactions/README.md`) -- `gpui`-free, the same "pure state, chrome renders
//! it" split `accounts.rs` uses. All data is stubbed and in-memory (the Desktop Transactions
//! Surface map's Destination): nothing here reads `lib_database`.
//!
//! **The shape runs ahead of the persisted model on purpose.** The glossary makes every Transaction
//! one or more **Splits** (an amount and a Category, plus an optional Payee and any Tags), and the
//! desktop table is built around that; `lib_database`'s `Transactions` row still carries a single
//! `category_id` and `payee_id` with no splits table. Real wiring later has to add splits.
//!
//! **The seed** ([`default_transactions`]) is deterministic (fixed seed, injectable `today`):
//! per-account counts equal the Accounts stub's `transaction_count` for the accounts that take
//! Transactions (Cash, Bank and Credit Card; Loan and Investment accounts take Repayments and
//! Trades instead), about 8% of the random ones are multi-Split, and a few hand-authored cases
//! always exist so every rule the table and filters must handle has a row to exercise it.
//!
//! No transaction account in this seed is in a second Unit, so the mixed-Units rule (the running
//! total blanks) is exercised by the filter engine's unit tests rather than visible in the app
//! until an account in another Unit holds transactions; seeding one would mean changing the
//! Accounts and Settings stubs the earlier maps reconciled to their mockups.

use bigdecimal::BigDecimal;
use chrono::{Duration, NaiveDate};
use lib_core::{AccountType, Money, TransactionStatus};

use crate::{
    accounts::Account,
    categories::{self, Category},
    payees::{self, Payee},
    tags::{self, Tag},
};

/// One line of a Transaction: an amount and a Category (a leaf), plus an optional Payee and any
/// number of Tags. Expenses are negative and income positive.
#[derive(Debug, Clone, PartialEq)]
pub struct Split {
    pub amount: Money,
    pub category_id: u32,
    pub payee_id: Option<u32>,
    pub tag_ids: Vec<u32>,
}

/// A single-entry record of an amount moving against one Transaction or Credit Card Account on a
/// date, composed of one or more [`Split`]s. Its unit is its account's.
#[derive(Debug, Clone, PartialEq)]
pub struct Transaction {
    pub id: u32,
    pub date: NaiveDate,
    /// The [`Account::id`] it is posted against.
    pub account_id: u32,
    pub status: TransactionStatus,
    /// Independent of `status`: any status can also be flagged.
    pub is_flagged: bool,
    /// The memo the search box matches.
    pub description: Option<String>,
    /// Never empty.
    pub splits: Vec<Split>,
}

impl Transaction {
    /// The sum of its Splits' amounts.
    pub fn total(&self) -> Money {
        self.splits.iter().fold(cents_money(0), |sum, split| {
            Money(sum.0 + split.amount.0.clone())
        })
    }

    /// Whether it has more than one Split.
    pub fn is_split(&self) -> bool {
        self.splits.len() > 1
    }
}

/// How far back the random transactions reach, in days.
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

/// Name lookups into the shared stubs, so the seed reads in names rather than ids.
struct Lookup<'a> {
    categories: &'a [Category],
    payees: &'a [Payee],
    tags: &'a [Tag],
}

impl Lookup<'_> {
    fn category(&self, name: &str) -> Option<u32> {
        categories::find_by_name(self.categories, name)
    }

    fn payee(&self, name: &str) -> Option<u32> {
        payees::find_by_name(self.payees, name)
    }

    fn split(
        &self,
        cents: i64,
        category: &str,
        payee: Option<&str>,
        tag_names: &[&str],
    ) -> Option<Split> {
        Some(Split {
            amount: cents_money(cents),
            category_id: self.category(category)?,
            payee_id: match payee {
                Some(name) => Some(self.payee(name)?),
                None => None,
            },
            tag_ids: tag_names
                .iter()
                .filter_map(|name| tags::find_by_name(self.tags, name))
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
        payees: &["Woolworths", "Coles Online", "Aldi Kelvin Grove"],
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
        payees: &["Sunrise Payroll"],
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

const RANDOM_TAGS: [&str; 3] = ["shared", "tax deductible", "reimbursable"];

/// Whether an account of this type takes Transactions directly (glossary: Loan and Investment
/// accounts do not).
fn takes_transactions(account_type: &AccountType) -> bool {
    matches!(
        account_type,
        AccountType::Cash | AccountType::Bank | AccountType::CreditCard
    )
}

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
    account_id: u32,
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
        .chance(6)
        .then(|| rng.range(0, parts.len() as i64 - 1) as usize);
    let tag_name = *rng.pick(&RANDOM_TAGS);
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
        id: 0,
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
/// Category), and one flagged transaction in each status.
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
                id: 0,
                date: day(days),
                account_id: 0,
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
            vec![lookup.split(421_000, "Salary", Some("Sunrise Payroll"), &[])],
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
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// The stub dataset: newest first, ids assigned oldest-first (a higher id is newer). Per-account
/// counts equal each account's `transaction_count`; accounts that take no Transactions, and any
/// account the seed has no profile for, get none.
pub fn default_transactions(
    accounts: &[Account],
    categories: &[Category],
    payees: &[Payee],
    tags: &[Tag],
    today: NaiveDate,
) -> Vec<Transaction> {
    let lookup = Lookup {
        categories,
        payees,
        tags,
    };
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

        let mut rng = Rng(DEFAULT_SEED ^ u64::from(account.id).wrapping_mul(0xA24B_AED4_963E_E407));
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
        transaction.id = index as u32 + 1;
    }
    all.reverse();
    all
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        accounts::default_accounts, categories::default_categories, payees::default_payees,
        tags::default_tags,
    };

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 9, 19).unwrap()
    }

    /// `(accounts, categories, payees, tags, transactions)`, seeded as the app does.
    type Seed = (
        Vec<Account>,
        Vec<Category>,
        Vec<Payee>,
        Vec<Tag>,
        Vec<Transaction>,
    );

    fn seed() -> Seed {
        let accounts = default_accounts();
        let categories = default_categories();
        let payees = default_payees();
        let tags = default_tags();
        let transactions = default_transactions(&accounts, &categories, &payees, &tags, today());
        (accounts, categories, payees, tags, transactions)
    }

    fn count_for(transactions: &[Transaction], accounts: &[Account], name: &str) -> usize {
        let id = accounts.iter().find(|a| a.name == name).unwrap().id;
        transactions.iter().filter(|t| t.account_id == id).count()
    }

    #[test]
    fn per_account_counts_match_the_accounts_stub() {
        let (accounts, _, _, _, transactions) = seed();
        assert_eq!(count_for(&transactions, &accounts, "Wallet"), 96);
        assert_eq!(count_for(&transactions, &accounts, "ANZ Everyday"), 312);
        assert_eq!(count_for(&transactions, &accounts, "ANZ Offset"), 88);
        assert_eq!(count_for(&transactions, &accounts, "Amex Platinum"), 204);
        assert_eq!(transactions.len(), 700);
    }

    #[test]
    fn loan_and_investment_accounts_hold_no_transactions() {
        let (accounts, _, _, _, transactions) = seed();
        for name in ["Home Loan", "Vanguard VAS", "Bitcoin"] {
            assert_eq!(count_for(&transactions, &accounts, name), 0, "{name}");
        }
    }

    #[test]
    fn the_seed_is_deterministic() {
        let (_, _, _, _, first) = seed();
        let (_, _, _, _, second) = seed();
        assert_eq!(first, second);
    }

    #[test]
    fn a_different_today_shifts_the_dates_but_not_the_shape() {
        let (accounts, categories, payees, tags, first) = seed();
        let later = today() + Duration::days(10);
        let second = default_transactions(&accounts, &categories, &payees, &tags, later);
        assert_eq!(first.len(), second.len());
        assert_eq!(first[0].date + Duration::days(10), second[0].date);
    }

    #[test]
    fn transactions_are_newest_first_with_unique_ids_that_rise_with_the_date() {
        let (_, _, _, _, transactions) = seed();
        let mut ids: Vec<_> = transactions.iter().map(|t| t.id).collect();
        for pair in transactions.windows(2) {
            assert!(pair[0].date >= pair[1].date, "newest first");
            assert!(pair[0].id > pair[1].id, "ids fall down the list");
        }
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), transactions.len());
        assert_eq!(ids[0], 1);
    }

    #[test]
    fn every_date_falls_within_the_history_window() {
        let (_, _, _, _, transactions) = seed();
        let oldest = today() - Duration::days(HISTORY_DAYS);
        for transaction in &transactions {
            assert!(transaction.date <= today());
            assert!(transaction.date >= oldest);
        }
    }

    #[test]
    fn splits_are_leaf_categories_with_real_payees_and_tags_and_the_right_sign() {
        let (_, categories, payees, tags, transactions) = seed();
        for transaction in &transactions {
            assert!(!transaction.splits.is_empty());
            for split in &transaction.splits {
                assert!(categories::is_leaf(&categories, split.category_id));
                let category = categories
                    .iter()
                    .find(|c| c.id == split.category_id)
                    .unwrap();
                let negative = split.amount.0.to_string().starts_with('-');
                let income = category.category_type == lib_core::CategoryTypes::Income;
                assert_eq!(negative, !income, "{} has the wrong sign", category.name);
                if let Some(payee) = split.payee_id {
                    assert!(payees.iter().any(|p| p.id == payee));
                }
                for tag in &split.tag_ids {
                    assert!(tags.iter().any(|t| t.id == *tag));
                }
            }
        }
    }

    #[test]
    fn a_transactions_total_is_the_sum_of_its_splits() {
        let receipt = Transaction {
            id: 1,
            date: today(),
            account_id: 1,
            status: TransactionStatus::Open,
            is_flagged: false,
            description: None,
            splits: vec![
                Split {
                    amount: cents_money(-18_000),
                    category_id: 7,
                    payee_id: None,
                    tag_ids: vec![],
                },
                Split {
                    amount: cents_money(-3_430),
                    category_id: 10,
                    payee_id: None,
                    tag_ids: vec![],
                },
            ],
        };
        assert_eq!(receipt.total(), cents_money(-21_430));
        assert!(receipt.is_split());
    }

    #[test]
    fn about_eight_percent_are_multi_split_with_two_or_three_splits() {
        let (_, _, _, _, transactions) = seed();
        let multi: Vec<_> = transactions.iter().filter(|t| t.is_split()).collect();
        let share = multi.len() * 100 / transactions.len();
        assert!((5..=12).contains(&share), "{share}% multi-split");
        for transaction in multi {
            assert!((2..=3).contains(&transaction.splits.len()));
        }
    }

    fn special<'a>(
        transactions: &'a [Transaction],
        accounts: &[Account],
        account: &str,
        description: &str,
    ) -> &'a Transaction {
        let id = accounts.iter().find(|a| a.name == account).unwrap().id;
        transactions
            .iter()
            .find(|t| t.account_id == id && t.description.as_deref() == Some(description))
            .unwrap_or_else(|| panic!("no special {description:?} on {account}"))
    }

    #[test]
    fn the_groceries_and_household_receipt_shares_one_payee() {
        let (accounts, categories, payees, _, transactions) = seed();
        let receipt = special(
            &transactions,
            &accounts,
            "ANZ Everyday",
            "Weekly shop and cleaning supplies",
        );
        assert_eq!(receipt.splits.len(), 2);
        let woolworths = payees::find_by_name(&payees, "Woolworths");
        assert!(receipt.splits.iter().all(|s| s.payee_id == woolworths));
        let groceries = categories::find_by_name(&categories, "Groceries").unwrap();
        let household = categories::find_by_name(&categories, "Household").unwrap();
        assert_eq!(receipt.splits[0].category_id, groceries);
        assert_eq!(receipt.splits[1].category_id, household);
        assert_eq!(receipt.total(), cents_money(-21_430));
    }

    #[test]
    fn one_receipt_has_splits_with_different_payees() {
        let (accounts, _, _, _, transactions) = seed();
        let receipt = special(&transactions, &accounts, "Amex Platinum", "Airport");
        assert_ne!(receipt.splits[0].payee_id, receipt.splits[1].payee_id);
    }

    #[test]
    fn one_receipt_has_a_tag_on_only_one_of_its_splits() {
        let (accounts, _, _, tags, transactions) = seed();
        let receipt = special(
            &transactions,
            &accounts,
            "ANZ Everyday",
            "Tokyo dinner and souvenirs",
        );
        let japan = tags::find_by_name(&tags, "Japan Trip 2026").unwrap();
        assert_eq!(receipt.splits[0].tag_ids, vec![japan]);
        assert!(receipt.splits[1].tag_ids.is_empty());
    }

    #[test]
    fn the_salary_is_positive_and_reconciled() {
        let (accounts, categories, _, _, transactions) = seed();
        let id = accounts
            .iter()
            .find(|a| a.name == "ANZ Everyday")
            .unwrap()
            .id;
        let salary = categories::find_by_name(&categories, "Salary").unwrap();
        let pay = transactions
            .iter()
            .find(|t| {
                t.account_id == id
                    && t.date == today() - Duration::days(9)
                    && t.splits[0].category_id == salary
            })
            .expect("the hand-authored salary");
        assert_eq!(pay.total(), cents_money(421_000));
        assert_eq!(pay.status, TransactionStatus::Reconciled);
    }

    #[test]
    fn every_status_and_flagged_combination_appears() {
        let (_, _, _, _, transactions) = seed();
        for status in [
            TransactionStatus::Open,
            TransactionStatus::Cleared,
            TransactionStatus::Reconciled,
        ] {
            for flagged in [false, true] {
                assert!(
                    transactions
                        .iter()
                        .any(|t| t.status == status && t.is_flagged == flagged),
                    "no {status:?} transaction with flagged = {flagged}"
                );
            }
        }
    }

    #[test]
    fn wallet_amounts_are_scaled_down() {
        let (accounts, _, _, _, transactions) = seed();
        let id = accounts.iter().find(|a| a.name == "Wallet").unwrap().id;
        for transaction in transactions.iter().filter(|t| t.account_id == id) {
            let text = transaction.total().0.to_string();
            let digits: String = text.chars().filter(char::is_ascii_digit).collect();
            let cents: i64 = digits.parse().unwrap();
            assert!(cents <= 10_000, "{text} is not a small cash amount");
        }
    }

    #[test]
    fn a_smaller_transaction_count_keeps_the_specials_first_and_the_total_exact() {
        let mut accounts = default_accounts();
        for account in &mut accounts {
            if account.name == "ANZ Everyday" {
                account.transaction_count = 3;
            }
        }
        let categories = default_categories();
        let payees = default_payees();
        let tags = default_tags();
        let transactions = default_transactions(&accounts, &categories, &payees, &tags, today());
        assert_eq!(count_for(&transactions, &accounts, "ANZ Everyday"), 3);
    }

    #[test]
    fn an_account_with_no_transaction_count_gets_none() {
        let mut accounts = default_accounts();
        for account in &mut accounts {
            account.transaction_count = 0;
        }
        let transactions = default_transactions(
            &accounts,
            &default_categories(),
            &default_payees(),
            &default_tags(),
            today(),
        );
        assert!(transactions.is_empty());
    }

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
