//! # Unit Kind Domain Module
//!
//! Descriptive grouping for a [`Unit`](crate) — fiat currency, crypto, a tradeable stock,
//! a precious metal, or anything else FR.1's "etc." covers. Purely a display/grouping aid;
//! Personal Ledger does not treat any kind specially (no cross-Unit conversion in V1 — see
//! `CONTEXT.md`'s Unit glossary entry).

/// One of the descriptive groupings a Unit can carry.
#[derive(Debug, Clone, Default, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize)]
pub enum UnitKind {
    /// Government-issued currency (e.g. AUD, USD, JPY).
    #[default]
    Fiat,

    /// A cryptocurrency (e.g. BTC, ETH).
    Crypto,

    /// A tradeable equity/stock ticker (e.g. AAPL).
    Stock,

    /// A precious metal (e.g. gold, silver), typically traded by weight.
    PreciousMetal,

    /// Anything not covered by the above.
    Other,
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
            "fiat" => Ok(UnitKind::Fiat),
            "crypto" => Ok(UnitKind::Crypto),
            "stock" => Ok(UnitKind::Stock),
            "precious_metal" => Ok(UnitKind::PreciousMetal),
            "other" => Ok(UnitKind::Other),
            _ => Err(UnitKindError::InvalidUnitKind(s.to_string())),
        }
    }
}

impl UnitKind {
    /// Returns the string representation stored in the database.
    pub fn as_str(&self) -> &'static str {
        match self {
            UnitKind::Fiat => "fiat",
            UnitKind::Crypto => "crypto",
            UnitKind::Stock => "stock",
            UnitKind::PreciousMetal => "precious_metal",
            UnitKind::Other => "other",
        }
    }

    /// Returns every variant, useful for validation or a TUI picker.
    pub fn all() -> &'static [UnitKind] {
        &[
            UnitKind::Fiat,
            UnitKind::Crypto,
            UnitKind::Stock,
            UnitKind::PreciousMetal,
            UnitKind::Other,
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

impl sqlx::Type<sqlx::Sqlite> for UnitKind {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for UnitKind {
    fn decode(value: sqlx::sqlite::SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        use std::str::FromStr;
        Ok(UnitKind::from_str(&s)?)
    }
}

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
        assert_eq!(UnitKind::from_str("FIAT"), Ok(UnitKind::Fiat));
        assert_eq!(UnitKind::from_str("Crypto"), Ok(UnitKind::Crypto));
    }

    #[test]
    fn from_str_rejects_unknown_values() {
        assert!(UnitKind::from_str("bogus").is_err());
    }

    #[test]
    fn default_is_fiat() {
        assert_eq!(UnitKind::default(), UnitKind::Fiat);
    }

    #[test]
    fn mock_returns_a_valid_variant() {
        let kind = UnitKind::mock();
        assert!(UnitKind::all().contains(&kind));
    }
}
