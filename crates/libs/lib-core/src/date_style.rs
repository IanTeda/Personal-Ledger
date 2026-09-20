//! # Date Style Domain Module
//!
//! How a Client displays dates -- a Preference (ADR-0014, reshaped by ADR-0021). The Locale owns
//! the actual pattern; a date style only picks which of the Locale's date forms to show, or
//! `Iso` to override the Locale for both display and typed input. The Preference itself is
//! nullable (`Option<DateStyle>`): no value means "use the Locale's default".
//!
//! Stored as a fixed set of TEXT choices, matching `UnitKind`'s enum-over-TEXT shape: the known
//! set is small, and `sqlx`'s compile-time query checking wants fixed columns.

/// One of the date display styles a Client can choose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, serde::Deserialize, serde::Serialize)]
pub enum DateStyle {
    /// The Locale's short numeric form (e.g. `31/12/2026` in `en-AU`).
    Short,

    /// The Locale's medium form, with an abbreviated month name (e.g. `31 Dec 2026`).
    Medium,

    /// The Locale's long form, with the full month name (e.g. `31 December 2026`).
    Long,

    /// `2026-12-31` -- unambiguous, sorts lexically, and overrides the Locale for typed input.
    Iso,
}

/// Error type for [`DateStyle`] parsing operations.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum DateStyleError {
    /// The provided string is not a valid date style.
    #[error("Invalid date style: {0}")]
    InvalidDateStyle(String),
}

impl std::fmt::Display for DateStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for DateStyle {
    type Err = DateStyleError;

    /// Parses a string to a [`DateStyle`] variant (case-insensitive).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "short" => Ok(DateStyle::Short),
            "medium" => Ok(DateStyle::Medium),
            "long" => Ok(DateStyle::Long),
            "iso" => Ok(DateStyle::Iso),
            _ => Err(DateStyleError::InvalidDateStyle(s.to_string())),
        }
    }
}

impl DateStyle {
    /// Returns the string representation stored in the database.
    pub fn as_str(&self) -> &'static str {
        match self {
            DateStyle::Short => "short",
            DateStyle::Medium => "medium",
            DateStyle::Long => "long",
            DateStyle::Iso => "iso",
        }
    }

    /// Returns every variant, useful for validation or a TUI picker.
    pub fn all() -> &'static [DateStyle] {
        &[
            DateStyle::Short,
            DateStyle::Medium,
            DateStyle::Long,
            DateStyle::Iso,
        ]
    }

    /// Picks a random variant -- test-only mock data.
    pub fn mock() -> Self {
        use fake::Fake;

        let all = Self::all();
        let index: usize = (0..all.len()).fake();
        all[index]
    }
}

impl sqlx::Type<sqlx::Sqlite> for DateStyle {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for DateStyle {
    fn decode(value: sqlx::sqlite::SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        use std::str::FromStr;
        Ok(DateStyle::from_str(&s)?)
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for DateStyle {
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
        for style in DateStyle::all() {
            assert_eq!(&DateStyle::from_str(style.as_str()).unwrap(), style);
        }
    }

    #[test]
    fn from_str_is_case_insensitive() {
        assert_eq!(DateStyle::from_str("ISO"), Ok(DateStyle::Iso));
        assert_eq!(DateStyle::from_str("Medium"), Ok(DateStyle::Medium));
    }

    #[test]
    fn from_str_rejects_unknown_values() {
        assert_eq!(
            DateStyle::from_str("bogus"),
            Err(DateStyleError::InvalidDateStyle("bogus".to_string()))
        );
    }

    #[test]
    fn display_matches_as_str() {
        for style in DateStyle::all() {
            assert_eq!(style.to_string(), style.as_str());
        }
    }

    #[test]
    fn mock_returns_a_valid_variant() {
        let style = DateStyle::mock();
        assert!(DateStyle::all().contains(&style));
    }
}
