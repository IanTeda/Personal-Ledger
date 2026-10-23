//! # Payees Database Model
//!
//! Defines the `Payees` struct, one row of the `payees` table — a canonical, user-visible
//! name for who a Transaction's money moved to or from (FR.16/18/21a-e, CC-TUI-007). A
//! first-class entity as of [ADR-0012](../../../../../docs/adr/0012-payee-entity-with-rename-aliases.md),
//! replacing the earlier free-text `Transactions.payee` column.

/// Database row model representing one persisted Payee.
#[derive(Debug, sqlx::FromRow, serde::Deserialize, serde::Serialize, PartialEq, Clone)]
pub struct Payees {
    /// Unique time-ordered identifier for the Payee.
    pub id: lib_core::RowID,

    /// The Payee's current display name (case-insensitively unique — see the `payees.name`
    /// column's `COLLATE NOCASE`). Renaming changes this in place; see [`Self::rename`].
    pub name: String,

    /// Soft-delete flag — `false` excludes the Payee from suggestions, but existing
    /// Transactions referencing it remain valid.
    pub is_active: bool,

    /// UTC timestamp when the Payee was first created.
    pub created_on: chrono::DateTime<chrono::Utc>,

    /// UTC timestamp when the Payee was last modified.
    pub updated_on: chrono::DateTime<chrono::Utc>,
}

impl Payees {
    /// Generates a mock `Payees` instance with randomised test data.
    ///
    /// The name carries a `RowID` suffix because `payees.name` is UNIQUE COLLATE NOCASE:
    /// a bare company name collides often enough to flake multi-insert tests.
    #[cfg(test)]
    pub fn mock() -> Self {
        use crate::payees::PayeesBuilder;
        use fake::Fake;
        use fake::faker::company::en::CompanyName;

        let now = chrono::Utc::now();
        PayeesBuilder::new()
            .with_id(lib_core::RowID::mock())
            .with_name(format!(
                "{}-{}",
                CompanyName().fake::<String>(),
                lib_core::RowID::new()
            ))
            .with_is_active_opt(Some(true))
            .with_created_on_opt(Some(now))
            .with_updated_on_opt(Some(now))
            .build()
            .expect("Mock Payees should always build successfully")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_generates_valid_payee() {
        let payee = Payees::mock();
        assert!(!payee.name.is_empty());
        assert!(payee.is_active);
    }

    /// `payees.name` is UNIQUE COLLATE NOCASE, so a bare company name would make
    /// multi-insert tests flake on the constraint (the same class as #604).
    #[test]
    fn mock_names_are_unique_ignoring_case() {
        let payees: Vec<Payees> = (0..200).map(|_| Payees::mock()).collect();
        let names: std::collections::HashSet<_> =
            payees.iter().map(|p| p.name.to_lowercase()).collect();
        assert_eq!(names.len(), payees.len());
    }

    #[test]
    fn payees_struct_derives_work() {
        let a = Payees::mock();
        let b = a.clone();
        assert_eq!(a, b);

        let debug_str = format!("{:?}", a);
        assert!(debug_str.contains("Payees"));

        let json = serde_json::to_string(&a).unwrap();
        let deserialized: Payees = serde_json::from_str(&json).unwrap();
        assert_eq!(a, deserialized);
    }
}
