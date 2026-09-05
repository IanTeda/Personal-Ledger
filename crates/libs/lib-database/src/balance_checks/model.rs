//! # Balance Checks Database Model
//!
//! Defines the `BalanceChecks` struct, one row of the `balance_checks` table — a
//! point-in-time assertion of what an Account's Balance should be (FR.28-32, CC-TUI-013),
//! checked against the Account's own Balance as computed from its Transactions (see
//! `CONTEXT.md`'s Balance Check entry). Manual entry only here; CSV import (FR.33) is a
//! separate ticket (CC-TUI-012).

/// Database row model representing one persisted Balance Check.
#[derive(Debug, sqlx::FromRow, serde::Deserialize, serde::Serialize, PartialEq, Clone)]
pub struct BalanceChecks {
    /// Unique time-ordered identifier for the Balance Check.
    pub id: lib_core::RowID,

    /// The Account this Balance Check asserts a Balance for, fixed at creation — mirrors
    /// `Accounts::unit_id`'s immutability (FR.31 only allows changing date/amount).
    pub account_id: lib_core::RowID,

    /// The point in time this assertion is made as of.
    pub date: chrono::NaiveDate,

    /// The asserted Balance, in the linked Account's Unit.
    pub asserted_balance: lib_core::Money,

    /// UTC timestamp when the Balance Check was first created.
    pub created_on: chrono::DateTime<chrono::Utc>,

    /// UTC timestamp when the Balance Check was last modified.
    pub updated_on: chrono::DateTime<chrono::Utc>,
}

impl BalanceChecks {
    /// Generates a mock `BalanceChecks` instance with randomised test data. `account_id`
    /// must be supplied by the caller (a real, already-inserted Account — the foreign key is
    /// enforced).
    #[cfg(test)]
    pub fn mock(account_id: lib_core::RowID) -> Self {
        use crate::balance_checks::BalanceChecksBuilder;

        let now = chrono::Utc::now();
        BalanceChecksBuilder::new()
            .with_id(lib_core::RowID::mock())
            .with_account_id(account_id)
            .with_date(now.date_naive())
            .with_asserted_balance(lib_core::Money::mock())
            .with_created_on_opt(Some(now))
            .with_updated_on_opt(Some(now))
            .build()
            .expect("Mock BalanceChecks should always build successfully")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_generates_valid_balance_check() {
        let balance_check = BalanceChecks::mock(lib_core::RowID::new());
        assert!(balance_check.created_on <= chrono::Utc::now());
    }

    #[test]
    fn balance_checks_struct_derives_work() {
        let a = BalanceChecks::mock(lib_core::RowID::new());
        let b = a.clone();
        assert_eq!(a, b);

        let debug_str = format!("{:?}", a);
        assert!(debug_str.contains("BalanceChecks"));

        let json = serde_json::to_string(&a).unwrap();
        let deserialized: BalanceChecks = serde_json::from_str(&json).unwrap();
        assert_eq!(a, deserialized);
    }
}
