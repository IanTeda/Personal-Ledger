//! # Account Type Domain Module
//!
//! The fixed classification an [`Account`](crate) carries (FR.10, `docs/product-requirements.md`):
//! Cash, Bank, Credit Card, Investment, or Loan.

/// One of the five kinds of place value can be held or owed.
#[derive(Debug, Clone, Default, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize)]
pub enum AccountType {
    #[default]
    Cash,
    Bank,
    CreditCard,
    Investment,
    Loan,
}

/// Error type for [`AccountType`] parsing operations.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum AccountTypeError {
    /// The provided string is not a valid Account type.
    #[error("Invalid account type: {0}")]
    InvalidAccountType(String),
}

impl std::fmt::Display for AccountType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for AccountType {
    type Err = AccountTypeError;

    /// Parses a string to an [`AccountType`] variant (case-insensitive).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "cash" => Ok(AccountType::Cash),
            "bank" => Ok(AccountType::Bank),
            "credit_card" => Ok(AccountType::CreditCard),
            "investment" => Ok(AccountType::Investment),
            "loan" => Ok(AccountType::Loan),
            _ => Err(AccountTypeError::InvalidAccountType(s.to_string())),
        }
    }
}

impl AccountType {
    /// Returns the string representation stored in the database.
    pub fn as_str(&self) -> &'static str {
        match self {
            AccountType::Cash => "cash",
            AccountType::Bank => "bank",
            AccountType::CreditCard => "credit_card",
            AccountType::Investment => "investment",
            AccountType::Loan => "loan",
        }
    }

    /// Returns every variant, useful for validation or a TUI picker.
    pub fn all() -> &'static [AccountType] {
        &[
            AccountType::Cash,
            AccountType::Bank,
            AccountType::CreditCard,
            AccountType::Investment,
            AccountType::Loan,
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

impl sqlx::Type<sqlx::Sqlite> for AccountType {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for AccountType {
    fn decode(value: sqlx::sqlite::SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        use std::str::FromStr;
        Ok(AccountType::from_str(&s)?)
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for AccountType {
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
        for account_type in AccountType::all() {
            assert_eq!(
                &AccountType::from_str(account_type.as_str()).unwrap(),
                account_type
            );
        }
    }

    #[test]
    fn from_str_is_case_insensitive() {
        assert_eq!(AccountType::from_str("CASH"), Ok(AccountType::Cash));
        assert_eq!(
            AccountType::from_str("Credit_Card"),
            Ok(AccountType::CreditCard)
        );
    }

    #[test]
    fn from_str_rejects_unknown_values() {
        assert!(AccountType::from_str("bogus").is_err());
    }

    #[test]
    fn default_is_cash() {
        assert_eq!(AccountType::default(), AccountType::Cash);
    }

    #[test]
    fn mock_returns_a_valid_variant() {
        let account_type = AccountType::mock();
        assert!(AccountType::all().contains(&account_type));
    }
}
