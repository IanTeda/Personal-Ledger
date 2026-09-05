//! # Transaction Status Domain Module
//!
//! Where a Transaction sits in the reconciliation workflow (FR.16/19, `CONTEXT.md`'s
//! Transaction Status entry): Open, Cleared, or Reconciled. Independent of the Flagged
//! marker — see `CONTEXT.md`, not a fourth status here.

/// One of the three reconciliation-workflow states a Transaction carries.
#[derive(Debug, Clone, Default, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize)]
pub enum TransactionStatus {
    /// Recorded but not yet confirmed against any external source.
    #[default]
    Open,

    /// Confirmed as having occurred, but not yet checked off during a formal
    /// reconciliation.
    Cleared,

    /// Matched against an account statement and confirmed as part of the total it
    /// asserts — the most-confirmed status. A Reconciled Transaction's other fields
    /// cannot be updated until its status is first moved back to Open or Cleared.
    Reconciled,
}

/// Error type for [`TransactionStatus`] parsing operations.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum TransactionStatusError {
    /// The provided string is not a valid Transaction Status.
    #[error("Invalid transaction status: {0}")]
    InvalidTransactionStatus(String),
}

impl std::fmt::Display for TransactionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for TransactionStatus {
    type Err = TransactionStatusError;

    /// Parses a string to a [`TransactionStatus`] variant (case-insensitive).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "open" => Ok(TransactionStatus::Open),
            "cleared" => Ok(TransactionStatus::Cleared),
            "reconciled" => Ok(TransactionStatus::Reconciled),
            _ => Err(TransactionStatusError::InvalidTransactionStatus(s.to_string())),
        }
    }
}

impl TransactionStatus {
    /// Returns the string representation stored in the database.
    pub fn as_str(&self) -> &'static str {
        match self {
            TransactionStatus::Open => "open",
            TransactionStatus::Cleared => "cleared",
            TransactionStatus::Reconciled => "reconciled",
        }
    }

    /// Returns every variant, useful for validation or a TUI picker.
    pub fn all() -> &'static [TransactionStatus] {
        &[
            TransactionStatus::Open,
            TransactionStatus::Cleared,
            TransactionStatus::Reconciled,
        ]
    }

    /// Picks a random variant — test-only mock data.
    pub fn mock() -> Self {
        use fake::Fake;

        let all = Self::all();
        let index: usize = (0..all.len()).fake();
        all[index].clone()
    }
}

impl sqlx::Type<sqlx::Sqlite> for TransactionStatus {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for TransactionStatus {
    fn decode(value: sqlx::sqlite::SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        use std::str::FromStr;
        Ok(TransactionStatus::from_str(&s)?)
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for TransactionStatus {
    fn encode_by_ref(
        &self,
        buf: &mut <sqlx::Sqlite as sqlx::Database>::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        <String as sqlx::Encode<'q, sqlx::Sqlite>>::encode(self.as_str().to_string(), buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn round_trips_through_as_str_and_from_str() {
        for status in TransactionStatus::all() {
            assert_eq!(&TransactionStatus::from_str(status.as_str()).unwrap(), status);
        }
    }

    #[test]
    fn from_str_is_case_insensitive() {
        assert_eq!(TransactionStatus::from_str("OPEN"), Ok(TransactionStatus::Open));
        assert_eq!(TransactionStatus::from_str("Reconciled"), Ok(TransactionStatus::Reconciled));
    }

    #[test]
    fn from_str_rejects_unknown_values() {
        assert!(TransactionStatus::from_str("bogus").is_err());
    }

    #[test]
    fn default_is_open() {
        assert_eq!(TransactionStatus::default(), TransactionStatus::Open);
    }

    #[test]
    fn mock_returns_a_valid_variant() {
        let status = TransactionStatus::mock();
        assert!(TransactionStatus::all().contains(&status));
    }
}
