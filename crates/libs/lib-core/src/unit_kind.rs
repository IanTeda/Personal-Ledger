//! # Unit Kind Domain Module
//!
//! The fixed, descriptive grouping a [`Unit`](crate) carries (`GLOSSARY.md`'s Unit Kind). It groups
//! Units for display and matches a Unit to the Price Sources able to price it; Personal Ledger
//! never converts between Units by Kind (no cross-Unit conversion in V1).

/// One of the descriptive groupings a Unit can carry.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, serde::Deserialize, serde::Serialize,
)]
pub enum UnitKind {
    /// Fiat money (e.g. AUD, USD, JPY).
    #[default]
    Currency,

    /// Coins and tokens, stablecoins included (e.g. BTC, ETH, USDC).
    Crypto,

    /// Shares, ETFs, LICs, REITs, managed funds and employee shares or options (e.g. BHP, VAS).
    Equity,

    /// Bonds and other market-priced debt. A term deposit is money held in an Account, not a Unit.
    FixedIncome,

    /// Precious metals and other raw goods (e.g. XAU).
    Commodity,

    /// The catch-all for anything else a person tracks (e.g. loyalty points, store credit).
    Custom,
}

/// Error type for [`UnitKind`] parsing operations.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum UnitKindError {
    /// The provided string is not a valid Unit kind.
    #[error("Invalid unit kind: {0}")]
    InvalidUnitKind(String),
}

impl std::fmt::Display for UnitKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for UnitKind {
    type Err = UnitKindError;

    /// Parses a string to a [`UnitKind`] variant (case-insensitive).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "currency" => Ok(UnitKind::Currency),
            "crypto" => Ok(UnitKind::Crypto),
            "equity" => Ok(UnitKind::Equity),
            "fixed_income" => Ok(UnitKind::FixedIncome),
            "commodity" => Ok(UnitKind::Commodity),
            "custom" => Ok(UnitKind::Custom),
            _ => Err(UnitKindError::InvalidUnitKind(s.to_string())),
        }
    }
}

impl UnitKind {
    /// Returns the string representation stored in the database.
    pub fn as_str(&self) -> &'static str {
        match self {
            UnitKind::Currency => "currency",
            UnitKind::Crypto => "crypto",
            UnitKind::Equity => "equity",
            UnitKind::FixedIncome => "fixed_income",
            UnitKind::Commodity => "commodity",
            UnitKind::Custom => "custom",
        }
    }

    /// Returns every variant, useful for validation or a TUI picker.
    pub fn all() -> &'static [UnitKind] {
        &[
            UnitKind::Currency,
            UnitKind::Crypto,
            UnitKind::Equity,
            UnitKind::FixedIncome,
            UnitKind::Commodity,
            UnitKind::Custom,
        ]
    }

    /// Picks a random variant — test-only mock data.
    pub fn mock() -> Self {
        use fake::Fake;

        let all = Self::all();
        let index: usize = (0..all.len()).fake();
        all[index]
    }
}

#[cfg(feature = "sqlx")]
impl sqlx::Type<sqlx::Sqlite> for UnitKind {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

#[cfg(feature = "sqlx")]
impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for UnitKind {
    fn decode(value: sqlx::sqlite::SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        use std::str::FromStr;
        Ok(UnitKind::from_str(&s)?)
    }
}

#[cfg(feature = "sqlx")]
impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for UnitKind {
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
        for kind in UnitKind::all() {
            assert_eq!(&UnitKind::from_str(kind.as_str()).unwrap(), kind);
        }
    }

    #[test]
    fn from_str_is_case_insensitive() {
        assert_eq!(UnitKind::from_str("CURRENCY"), Ok(UnitKind::Currency));
        assert_eq!(UnitKind::from_str("Crypto"), Ok(UnitKind::Crypto));
    }

    #[test]
    fn from_str_rejects_unknown_values() {
        assert!(UnitKind::from_str("bogus").is_err());
    }

    #[test]
    fn from_str_rejects_the_old_names() {
        for old in ["fiat", "stock", "precious_metal", "other"] {
            assert!(UnitKind::from_str(old).is_err(), "{old}");
        }
    }

    #[test]
    fn there_are_exactly_six_kinds() {
        assert_eq!(UnitKind::all().len(), 6);
    }

    #[test]
    fn default_is_currency() {
        assert_eq!(UnitKind::default(), UnitKind::Currency);
    }

    #[test]
    fn mock_returns_a_valid_variant() {
        let kind = UnitKind::mock();
        assert!(UnitKind::all().contains(&kind));
    }
}
