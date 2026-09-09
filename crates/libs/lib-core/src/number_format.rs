//! # Number Format Domain Module
//!
//! How a Client displays amounts -- a Preference (ADR-0014) coupling the thousands and
//! decimal separator into one choice, since real locales always pair them (`1,234.56` vs
//! `1.234,56` vs `1 234,56`), never mix-and-match independently.

/// One of the thousands/decimal separator pairings a Client can display amounts with.
#[derive(Debug, Clone, Default, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize)]
pub enum NumberFormat {
    /// `1,234.56` -- comma thousands, period decimal (e.g. US/UK/AU convention).
    #[default]
    CommaThousandsPeriodDecimal,

    /// `1.234,56` -- period thousands, comma decimal (much of continental Europe).
    PeriodThousandsCommaDecimal,

    /// `1 234,56` -- space thousands, comma decimal (e.g. French convention).
    SpaceThousandsCommaDecimal,
}

/// Error type for [`NumberFormat`] parsing operations.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum NumberFormatError {
    /// The provided string is not a valid number format.
    #[error("Invalid number format: {0}")]
    InvalidNumberFormat(String),
}

impl std::fmt::Display for NumberFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for NumberFormat {
    type Err = NumberFormatError;

    /// Parses a string to a [`NumberFormat`] variant (case-insensitive).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "comma_thousands_period_decimal" => Ok(NumberFormat::CommaThousandsPeriodDecimal),
            "period_thousands_comma_decimal" => Ok(NumberFormat::PeriodThousandsCommaDecimal),
            "space_thousands_comma_decimal" => Ok(NumberFormat::SpaceThousandsCommaDecimal),
            _ => Err(NumberFormatError::InvalidNumberFormat(s.to_string())),
        }
    }
}

impl NumberFormat {
    /// Returns the string representation stored in the database.
    pub fn as_str(&self) -> &'static str {
        match self {
            NumberFormat::CommaThousandsPeriodDecimal => "comma_thousands_period_decimal",
            NumberFormat::PeriodThousandsCommaDecimal => "period_thousands_comma_decimal",
            NumberFormat::SpaceThousandsCommaDecimal => "space_thousands_comma_decimal",
        }
    }

    /// Returns the character used to group thousands.
    pub fn thousands_separator(&self) -> char {
        match self {
            NumberFormat::CommaThousandsPeriodDecimal => ',',
            NumberFormat::PeriodThousandsCommaDecimal => '.',
            NumberFormat::SpaceThousandsCommaDecimal => ' ',
        }
    }

    /// Returns the character used before the fractional part.
    pub fn decimal_separator(&self) -> char {
        match self {
            NumberFormat::CommaThousandsPeriodDecimal => '.',
            NumberFormat::PeriodThousandsCommaDecimal => ',',
            NumberFormat::SpaceThousandsCommaDecimal => ',',
        }
    }

    /// Returns every variant, useful for validation or a TUI picker.
    pub fn all() -> &'static [NumberFormat] {
        &[
            NumberFormat::CommaThousandsPeriodDecimal,
            NumberFormat::PeriodThousandsCommaDecimal,
            NumberFormat::SpaceThousandsCommaDecimal,
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

impl sqlx::Type<sqlx::Sqlite> for NumberFormat {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for NumberFormat {
    fn decode(value: sqlx::sqlite::SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        use std::str::FromStr;
        Ok(NumberFormat::from_str(&s)?)
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for NumberFormat {
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
        for format in NumberFormat::all() {
            assert_eq!(&NumberFormat::from_str(format.as_str()).unwrap(), format);
        }
    }

    #[test]
    fn from_str_is_case_insensitive() {
        assert_eq!(
            NumberFormat::from_str("COMMA_THOUSANDS_PERIOD_DECIMAL"),
            Ok(NumberFormat::CommaThousandsPeriodDecimal)
        );
    }

    #[test]
    fn from_str_rejects_unknown_values() {
        assert!(NumberFormat::from_str("bogus").is_err());
    }

    #[test]
    fn default_is_comma_thousands_period_decimal() {
        assert_eq!(
            NumberFormat::default(),
            NumberFormat::CommaThousandsPeriodDecimal
        );
    }

    #[test]
    fn separators_are_never_the_same_character() {
        for format in NumberFormat::all() {
            assert_ne!(format.thousands_separator(), format.decimal_separator());
        }
    }

    #[test]
    fn mock_returns_a_valid_variant() {
        let format = NumberFormat::mock();
        assert!(NumberFormat::all().contains(&format));
    }
}
