//! The Transaction record and its Splits (`docs/transactions.md`, the glossary's Transaction and
//! Split), `gpui`-free and I/O-free, shared by the Desktop and the TUI. Seeding, filtering and
//! rendering stay with the Client.
//!
//! A Transaction is a single-entry record of an amount against one Account on a date; its
//! Splits carry the amounts and Categories, and the Unit is its Account's.

use bigdecimal::BigDecimal;
use chrono::NaiveDate;
use lib_core::{AccountType, Money, TransactionStatus};

/// A zero amount in cents, the sum every Transaction's total folds from.
fn cents_money(cents: i64) -> Money {
    Money(BigDecimal::new(cents.into(), 2))
}

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
    /// The the Account id it is posted against.
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

/// Whether an account of this type takes Transactions directly (glossary: Loan and Investment
/// accounts do not).
pub fn takes_transactions(account_type: &AccountType) -> bool {
    matches!(
        account_type,
        AccountType::Cash | AccountType::Bank | AccountType::CreditCard
    )
}
