//! The Transaction record and its Splits (`docs/transactions.md`, the glossary's Transaction and
//! Split), `gpui`-free and I/O-free, shared by the Desktop and the TUI. Filtering and rendering
//! stay with the Client; the query engine behind the Transactions table is `query`.
//!
//! A ports-only VIPER module so far (`docs/development/viper-architecture.md`, #774): the
//! repository port and the read ports for the other domains' rows in `protocols`, which
//! `lib-database` implements, their in-memory twins and the stub seed in `fake`, and the contract
//! both keep in `contract`.
//!
//! A Transaction is a single-entry record of an amount against one Account on a date; its
//! Splits carry the amounts and Categories, and the Unit is its Account's.

#[cfg(feature = "contract-tests")]
#[expect(
    clippy::expect_used,
    reason = "a contract check is test code a consumer runs, and fails by panicking as a test does"
)]
pub mod contract;
pub mod fake;
pub mod protocols;
pub mod query;

pub use protocols::{
    TransactionAccount, TransactionAccounts, TransactionCategories, TransactionCategory,
    TransactionPayee, TransactionPayees, TransactionRepository, TransactionTag, TransactionTags,
    TransactionUnit, TransactionUnits,
};

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use lib_core::{
    AccountId, AccountType, CategoryId, Money, PayeeId, TagId, TransactionId, TransactionStatus,
};

/// A zero amount in cents, the sum every Transaction's total folds from.
fn cents_money(cents: i64) -> Money {
    Money(BigDecimal::new(cents.into(), 2))
}

/// One line of a Transaction: an amount and a Category (a leaf), plus an optional Payee and any
/// number of Tags. Expenses are negative and income positive.
#[derive(Debug, Clone, PartialEq)]
pub struct Split {
    pub amount: Money,
    pub category_id: CategoryId,
    pub payee_id: Option<PayeeId>,
    pub tag_ids: Vec<TagId>,
}

/// A single-entry record of an amount moving against one Transaction or Credit Card Account on a
/// date, composed of one or more [`Split`]s. Its unit is its account's.
#[derive(Debug, Clone, PartialEq)]
pub struct Transaction {
    pub id: TransactionId,
    pub date: NaiveDate,
    /// The the Account id it is posted against.
    pub account_id: AccountId,
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

/// Whether an account of this type takes Transactions directly (glossary: Loan and Investment
/// accounts do not).
pub fn takes_transactions(account_type: &AccountType) -> bool {
    matches!(
        account_type,
        AccountType::Cash | AccountType::Bank | AccountType::CreditCard
    )
}
