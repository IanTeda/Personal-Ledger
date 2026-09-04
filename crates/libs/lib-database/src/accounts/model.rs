//! # Accounts Database Model
//!
//! Defines the `Accounts` struct, one row of the `accounts` table — a place where value is
//! held or owed (Cash, Bank, Credit Card, Investment, or Loan; FR.10-15, CC-TUI-008),
//! denominated in exactly one Unit, fixed at creation. Its running Balance is computed on
//! read from `starting_balance` plus its Transactions (compute-on-read, see "Decide the TUI
//! app's data model and persistence layer") — no stored `balance` column here.

/// Database row model representing one persisted Account.
#[derive(Debug, sqlx::FromRow, serde::Deserialize, serde::Serialize, PartialEq, Clone)]
pub struct Accounts {
    /// Unique time-ordered identifier for the Account.
    pub id: lib_core::RowID,

    /// Human-readable display name (e.g. "Everyday Spending", "Home Loan").
    pub name: String,

    /// One of Cash, Bank, Credit Card, Investment, or Loan.
    pub account_type: lib_core::AccountType,

    /// The Unit this Account is denominated in, fixed at creation. References `units.id`
    /// (enforced by SQLite's foreign_keys pragma) — every Transaction against this Account
    /// is measured in the same Unit.
    pub unit_id: lib_core::RowID,

    /// The Account's balance at creation, in its own Unit. Combined with the sum of its
    /// Transactions to compute the current Balance on read.
    pub starting_balance: lib_core::Money,

    /// Soft-delete flag — `false` means the Account should not be used for new
    /// Transactions, but existing ones remain valid.
    pub is_active: bool,

    /// UTC timestamp when the Account was first created.
    pub created_on: chrono::DateTime<chrono::Utc>,

    /// UTC timestamp when the Account was last modified.
    pub updated_on: chrono::DateTime<chrono::Utc>,
}

impl Accounts {
    /// Generates a mock `Accounts` instance with randomised test data. `unit_id` must be
    /// supplied by the caller (a real, already-inserted Unit — the foreign key is enforced).
    #[cfg(test)]
    pub fn mock(unit_id: lib_core::RowID) -> Self {
        use crate::accounts::AccountsBuilder;
        use fake::Fake;
        use fake::faker::company::en::CompanyName;

        let now = chrono::Utc::now();
        AccountsBuilder::new()
            .with_id(lib_core::RowID::mock())
            .with_name(CompanyName().fake::<String>())
            .with_account_type(lib_core::AccountType::mock())
            .with_unit_id(unit_id)
            .with_starting_balance(lib_core::Money::mock())
            .with_is_active_opt(Some(true))
            .with_created_on_opt(Some(now))
            .with_updated_on_opt(Some(now))
            .build()
            .expect("Mock Accounts should always build successfully")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_generates_valid_account() {
        let account = Accounts::mock(lib_core::RowID::new());
        assert!(!account.name.is_empty());
        assert!(account.is_active);
    }

    #[test]
    fn accounts_struct_derives_work() {
        let a = Accounts::mock(lib_core::RowID::new());
        let b = a.clone();
        assert_eq!(a, b);

        let debug_str = format!("{:?}", a);
        assert!(debug_str.contains("Accounts"));

        let json = serde_json::to_string(&a).unwrap();
        let deserialized: Accounts = serde_json::from_str(&json).unwrap();
        assert_eq!(a, deserialized);
    }
}
