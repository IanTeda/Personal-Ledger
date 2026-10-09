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

/// The Transactions the Desktop and the TUI read and write, the seam between a Client and its
/// data. A Client owns one service and reads it through its own store; mutations go through
/// [`TransactionService::edit`] so every write is one closure a store can notify around.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TransactionService {
    transactions: Vec<Transaction>,
}

impl TransactionService {
    /// A service over these rows, in the order they are given (newest first for the stub data).
    pub fn from_rows(transactions: Vec<Transaction>) -> Self {
        Self { transactions }
    }

    /// Every Transaction, in the service's order.
    pub fn transactions(&self) -> &[Transaction] {
        &self.transactions
    }

    /// Edits the rows in one closure, so a caller can make a multi-step change (a merge, a delete
    /// by account) without a second borrow.
    pub fn edit<R>(&mut self, change: impl FnOnce(&mut Vec<Transaction>) -> R) -> R {
        change(&mut self.transactions)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn transaction(id: u32, account_id: u32) -> Transaction {
        Transaction {
            id,
            date: NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"),
            account_id,
            status: TransactionStatus::default(),
            is_flagged: false,
            description: None,
            splits: vec![Split {
                amount: cents_money(100),
                category_id: 1,
                payee_id: None,
                tag_ids: Vec::new(),
            }],
        }
    }

    #[test]
    fn from_rows_keeps_the_given_order() {
        let service = TransactionService::from_rows(vec![transaction(2, 1), transaction(1, 1)]);
        let ids: Vec<u32> = service.transactions().iter().map(|t| t.id).collect();
        assert_eq!(ids, vec![2, 1]);
    }

    #[test]
    fn edit_applies_the_change_to_the_rows() {
        let mut service = TransactionService::from_rows(vec![transaction(1, 1), transaction(2, 9)]);
        let removed = service.edit(|rows| {
            let before = rows.len();
            rows.retain(|t| t.account_id != 9);
            before - rows.len()
        });
        assert_eq!(removed, 1);
        assert_eq!(service.transactions().len(), 1);
    }
}
