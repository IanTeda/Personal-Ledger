//! # Transactions Database Model
//!
//! Defines the `Transactions` struct, one row of the `transactions` table — a single-entry
//! record of an amount moving against exactly one Account and one Category (FR.16-21,
//! CC-TUI-009). No `created_on` column: FR.21 says the UUIDv7 `id` itself determines
//! creation date. Personal Ledger is deliberately single-entry, not double-entry (see
//! `CONTEXT.md`'s Transaction entry and [ADR-0001](../../../../../docs/adr/0001-single-entry-not-double-entry.md)).

/// Database row model representing one persisted Transaction.
#[derive(Debug, sqlx::FromRow, serde::Deserialize, serde::Serialize, PartialEq, Clone)]
pub struct Transactions {
    /// Unique time-ordered identifier for the Transaction — also determines its creation
    /// date (FR.21), so there's no separate `created_on` column.
    pub id: lib_core::RowID,

    /// The Transaction's effective/value date, distinct from `id`'s creation timestamp.
    pub date: chrono::NaiveDate,

    /// The amount, in the linked Account's Unit. Positive increases the Account's Balance,
    /// negative decreases it — the sign is the transaction's type, there's no separate
    /// income/expense field.
    pub amount: lib_core::Money,

    /// The exactly-one Category this Transaction is classified under.
    pub category_id: lib_core::RowID,

    /// The exactly-one Account this Transaction is posted against.
    pub account_id: lib_core::RowID,

    /// The optional Payee money moved to or from. A first-class entity as of ADR-0012 —
    /// see `CONTEXT.md`'s Payee entry — resolved or auto-created from typed text at save
    /// time, not stored as free text here.
    pub payee_id: Option<lib_core::RowID>,

    /// Optional free-text note.
    pub description: Option<String>,

    /// Where this Transaction sits in the reconciliation workflow.
    pub status: lib_core::TransactionStatus,

    /// Independent of `status` — a marker for follow-up or review.
    pub is_flagged: bool,

    /// UTC timestamp when the Transaction was last modified.
    pub updated_on: chrono::DateTime<chrono::Utc>,
}

impl Transactions {
    /// Generates a mock `Transactions` instance with randomised test data. `category_id`
    /// and `account_id` must be supplied by the caller (real, already-inserted rows — both
    /// foreign keys are enforced).
    #[cfg(test)]
    pub fn mock(category_id: lib_core::RowID, account_id: lib_core::RowID) -> Self {
        use crate::transactions::TransactionsBuilder;
        use fake::Fake;
        use fake::faker::lorem::en::Words;

        let description = {
            let words: Vec<String> = Words(2..5).fake();
            words.join(" ")
        };

        TransactionsBuilder::new()
            .with_id(lib_core::RowID::mock())
            .with_date(chrono::Utc::now().date_naive())
            .with_amount(lib_core::Money::mock())
            .with_category_id(category_id)
            .with_account_id(account_id)
            .with_payee_id_opt(None)
            .with_description_opt(Some(description))
            .with_status(lib_core::TransactionStatus::mock())
            .with_is_flagged_opt(Some(false))
            .with_updated_on_opt(Some(chrono::Utc::now()))
            .build()
            .expect("Mock Transactions should always build successfully")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_generates_valid_transaction() {
        let transaction = Transactions::mock(lib_core::RowID::new(), lib_core::RowID::new());
        assert!(!transaction.is_flagged);
    }

    #[test]
    fn transactions_struct_derives_work() {
        let a = Transactions::mock(lib_core::RowID::new(), lib_core::RowID::new());
        let b = a.clone();
        assert_eq!(a, b);

        let debug_str = format!("{:?}", a);
        assert!(debug_str.contains("Transactions"));

        let json = serde_json::to_string(&a).unwrap();
        let deserialized: Transactions = serde_json::from_str(&json).unwrap();
        assert_eq!(a, deserialized);
    }
}
