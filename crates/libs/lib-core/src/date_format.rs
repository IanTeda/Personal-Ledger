//! # Date Format Domain Module
//!
//! How a Client displays dates -- a Preference (ADR-0014), stored as a fixed set of choices
//! rather than a free-form strftime string, matching `UnitKind`'s enum-over-TEXT shape: the
//! known set is small, and `sqlx`'s compile-time query checking wants fixed columns.

/// One of the date display formats a Client can show.
#[derive(Debug, Clone, Default, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize)]
pub enum DateFormat {
    /// `31/12/2026` -- day first, the common non-US convention.
    #[default]
    DayMonthYear,

    /// `12/31/2026` -- month first, the US convention.
    MonthDayYear,

    /// `2026-12-31` -- unambiguous, sorts lexically.
    Iso,
}

/// Error type for [`DateFormat`] parsing operations.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum DateFormatError {
    /// The provided string is not a valid date format.
    #[error("Invalid date format: {0}")]
    InvalidDateFormat(String),
}

impl std::fmt::Display for DateFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for DateFormat {
    type Err = DateFormatError;

    /// Parses a string to a [`DateFormat`] variant (case-insensitive).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "day_month_year" => Ok(DateFormat::DayMonthYear),
            "month_day_year" => Ok(DateFormat::MonthDayYear),
            "iso" => Ok(DateFormat::Iso),
            _ => Err(DateFormatError::InvalidDateFormat(s.to_string())),
        }
    }
}

impl DateFormat {
    /// Returns the string representation stored in the database.
    pub fn as_str(&self) -> &'static str {
        match self {
            DateFormat::DayMonthYear => "day_month_year",
            DateFormat::MonthDayYear => "month_day_year",
            DateFormat::Iso => "iso",
        }
    }

    /// Returns the `chrono` strftime pattern this format corresponds to.
    pub fn pattern(&self) -> &'static str {
        match self {
            DateFormat::DayMonthYear => "%d/%m/%Y",
            DateFormat::MonthDayYear => "%m/%d/%Y",
            DateFormat::Iso => "%Y-%m-%d",
        }
    }

    /// Returns every variant, useful for validation or a TUI picker.
    pub fn all() -> &'static [DateFormat] {
        &[
            DateFormat::DayMonthYear,
            DateFormat::MonthDayYear,
            DateFormat::Iso,
        ]
    }

    /// Picks a random variant -- test-only mock data.
    pub fn mock() -> Self {
        use fake::Fake;

        let all = Self::all();
        let index: usize = (0..all.len()).fake();
        all[index].clone()
    }
}

impl sqlx::Type<sqlx::Sqlite> for DateFormat {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for DateFormat {
    fn decode(value: sqlx::sqlite::SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        use std::str::FromStr;
        Ok(DateFormat::from_str(&s)?)
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for DateFormat {
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
        for format in DateFormat::all() {
            assert_eq!(&DateFormat::from_str(format.as_str()).unwrap(), format);
        }
    }

    #[test]
    fn from_str_is_case_insensitive() {
        assert_eq!(DateFormat::from_str("ISO"), Ok(DateFormat::Iso));
        assert_eq!(
            DateFormat::from_str("Day_Month_Year"),
            Ok(DateFormat::DayMonthYear)
        );
    }

    #[test]
    fn from_str_rejects_unknown_values() {
        assert!(DateFormat::from_str("bogus").is_err());
    }

    #[test]
    fn default_is_day_month_year() {
        assert_eq!(DateFormat::default(), DateFormat::DayMonthYear);
    }

    #[test]
    fn pattern_matches_expected_strftime() {
        assert_eq!(DateFormat::Iso.pattern(), "%Y-%m-%d");
        assert_eq!(DateFormat::DayMonthYear.pattern(), "%d/%m/%Y");
        assert_eq!(DateFormat::MonthDayYear.pattern(), "%m/%d/%Y");
    }

    #[test]
    fn mock_returns_a_valid_variant() {
        let format = DateFormat::mock();
        assert!(DateFormat::all().contains(&format));
    }
}
