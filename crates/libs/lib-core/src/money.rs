//! # Money Domain Type
//!
//! Exact-decimal wrapper around [`bigdecimal::BigDecimal`] for every amount a Transaction,
//! Account starting balance, Budget limit, or Balance Check asserts (NFR.1 reliability —
//! money is never handled as a float). `sqlx`'s own `bigdecimal` feature only maps
//! `BigDecimal` to Postgres' native `NUMERIC` type; SQLite has no equivalent native decimal
//! type (deliberately unsupported by `sqlx-sqlite`, see its `types` module docs), so `Money`
//! provides its own SQLite storage as exact-round-tripping text, the same pattern `UtcDateTime`
//! already uses in this crate for `chrono::DateTime<Utc>`.

use bigdecimal::BigDecimal;

/// An exact decimal amount, stored as text in SQLite to avoid `f64` rounding error.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Deserialize, serde::Serialize)]
pub struct Money(pub BigDecimal);

/// Error type for [`Money`] parsing operations.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum MoneyError {
    /// The provided string is not a valid decimal amount.
    #[error("Invalid money amount: {0}")]
    InvalidAmount(String),
}

impl std::fmt::Display for Money {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for Money {
    type Err = MoneyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<BigDecimal>()
            .map(Money)
            .map_err(|_| MoneyError::InvalidAmount(s.to_string()))
    }
}

impl From<BigDecimal> for Money {
    fn from(value: BigDecimal) -> Self {
        Money(value)
    }
}

impl From<Money> for BigDecimal {
    fn from(value: Money) -> Self {
        value.0
    }
}

impl Money {
    /// A fixed, deterministic mock amount — test-only.
    pub fn mock() -> Self {
        Money(BigDecimal::from(100))
    }
}

impl sqlx::Type<sqlx::Sqlite> for Money {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for Money {
    fn decode(value: sqlx::sqlite::SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        use std::str::FromStr;
        Ok(Money::from_str(&s)?)
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for Money {
    fn encode_by_ref(
        &self,
        buf: &mut <sqlx::Sqlite as sqlx::Database>::ArgumentBuffer<'q>,
    ) -> Result<sqlx::encode::IsNull, sqlx::error::BoxDynError> {
        <String as sqlx::Encode<'q, sqlx::Sqlite>>::encode(self.0.to_string(), buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn round_trips_through_display_and_from_str() {
        let money = Money::from_str("123.45").unwrap();
        assert_eq!(money.to_string(), "123.45");
    }

    #[test]
    fn rejects_a_non_numeric_string() {
        assert!(Money::from_str("not-a-number").is_err());
    }

    #[test]
    fn preserves_exact_precision_unlike_a_float() {
        // 0.1 + 0.2 famously isn't 0.3 in f64; BigDecimal-backed Money keeps it exact.
        let a = Money::from_str("0.1").unwrap();
        let b = Money::from_str("0.2").unwrap();
        let sum = Money(a.0 + b.0);
        assert_eq!(sum.to_string(), "0.3");
    }

    #[test]
    fn mock_produces_a_valid_amount() {
        let money = Money::mock();
        assert_eq!(money.to_string(), "100");
    }
}
