//! # Budget Period Domain Module
//!
//! How often a Budget's limit recurs (FR.23, `CONTEXT.md`'s Budget entry): Weekly, Monthly,
//! Quarterly, or Yearly. [`Self::current_bounds`] computes the current period's calendar
//! boundaries — always a fixed calendar boundary (ISO week Monday-start, calendar month,
//! calendar quarter, calendar year), never a rolling window anchored to the Budget's own
//! creation date, so every Budget shares one predictable period grid.

use chrono::{Datelike, NaiveDate};

/// How often a Budget's limit recurs.
#[derive(Debug, Clone, Default, PartialEq, PartialOrd, serde::Deserialize, serde::Serialize)]
pub enum BudgetPeriod {
    /// An ISO week, Monday to Sunday.
    Weekly,

    /// A calendar month.
    #[default]
    Monthly,

    /// A calendar quarter (Jan-Mar, Apr-Jun, Jul-Sep, Oct-Dec).
    Quarterly,

    /// A calendar year.
    Yearly,
}

/// Error type for [`BudgetPeriod`] parsing operations.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum BudgetPeriodError {
    /// The provided string is not a valid Budget Period.
    #[error("Invalid budget period: {0}")]
    InvalidBudgetPeriod(String),
}

impl std::fmt::Display for BudgetPeriod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for BudgetPeriod {
    type Err = BudgetPeriodError;

    /// Parses a string to a [`BudgetPeriod`] variant (case-insensitive).
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "weekly" => Ok(BudgetPeriod::Weekly),
            "monthly" => Ok(BudgetPeriod::Monthly),
            "quarterly" => Ok(BudgetPeriod::Quarterly),
            "yearly" => Ok(BudgetPeriod::Yearly),
            _ => Err(BudgetPeriodError::InvalidBudgetPeriod(s.to_string())),
        }
    }
}

impl BudgetPeriod {
    /// Returns the string representation stored in the database.
    pub fn as_str(&self) -> &'static str {
        match self {
            BudgetPeriod::Weekly => "weekly",
            BudgetPeriod::Monthly => "monthly",
            BudgetPeriod::Quarterly => "quarterly",
            BudgetPeriod::Yearly => "yearly",
        }
    }

    /// Returns every variant, useful for validation or a TUI picker.
    pub fn all() -> &'static [BudgetPeriod] {
        &[
            BudgetPeriod::Weekly,
            BudgetPeriod::Monthly,
            BudgetPeriod::Quarterly,
            BudgetPeriod::Yearly,
        ]
    }

    /// Picks a random variant — test-only mock data.
    pub fn mock() -> Self {
        use fake::Fake;

        let all = Self::all();
        let index: usize = (0..all.len()).fake();
        all[index].clone()
    }

    /// The inclusive `[start, end]` calendar bounds of the period containing `today`.
    pub fn current_bounds(&self, today: NaiveDate) -> (NaiveDate, NaiveDate) {
        match self {
            BudgetPeriod::Weekly => {
                let days_since_monday = today.weekday().num_days_from_monday() as i64;
                let start = today - chrono::Duration::days(days_since_monday);
                let end = start + chrono::Duration::days(6);
                (start, end)
            }
            BudgetPeriod::Monthly => {
                let start = NaiveDate::from_ymd_opt(today.year(), today.month(), 1)
                    .expect("day 1 is always valid");
                let end = last_day_of_month(today.year(), today.month());
                (start, end)
            }
            BudgetPeriod::Quarterly => {
                let quarter_start_month = ((today.month0() / 3) * 3) + 1;
                let start = NaiveDate::from_ymd_opt(today.year(), quarter_start_month, 1)
                    .expect("day 1 is always valid");
                let end = last_day_of_month(today.year(), quarter_start_month + 2);
                (start, end)
            }
            BudgetPeriod::Yearly => {
                let start =
                    NaiveDate::from_ymd_opt(today.year(), 1, 1).expect("Jan 1 is always valid");
                let end =
                    NaiveDate::from_ymd_opt(today.year(), 12, 31).expect("Dec 31 is always valid");
                (start, end)
            }
        }
    }

    /// How far through the current period `today` is, as a fraction from `0.0` (the first
    /// day) to `1.0` (the last day) — the "today" line's position on a progress bar.
    pub fn progress_fraction(&self, today: NaiveDate) -> f64 {
        let (start, end) = self.current_bounds(today);
        let total_days = (end - start).num_days().max(1) as f64;
        let elapsed_days = (today - start).num_days().clamp(0, total_days as i64) as f64;
        elapsed_days / total_days
    }
}

/// The last calendar day of `year`/`month` (1-indexed month, rolling `month` past 12 into
/// the next year as needed — used for quarter-end calculation).
fn last_day_of_month(year: i32, month: u32) -> NaiveDate {
    let (next_year, next_month) = if month >= 12 {
        (year + 1, month - 11)
    } else {
        (year, month + 1)
    };
    NaiveDate::from_ymd_opt(next_year, next_month, 1).expect("a valid first-of-month always exists")
        - chrono::Duration::days(1)
}

impl sqlx::Type<sqlx::Sqlite> for BudgetPeriod {
    fn type_info() -> sqlx::sqlite::SqliteTypeInfo {
        <String as sqlx::Type<sqlx::Sqlite>>::type_info()
    }
}

impl<'r> sqlx::Decode<'r, sqlx::Sqlite> for BudgetPeriod {
    fn decode(value: sqlx::sqlite::SqliteValueRef<'r>) -> Result<Self, sqlx::error::BoxDynError> {
        let s = <String as sqlx::Decode<sqlx::Sqlite>>::decode(value)?;
        use std::str::FromStr;
        Ok(BudgetPeriod::from_str(&s)?)
    }
}

impl<'q> sqlx::Encode<'q, sqlx::Sqlite> for BudgetPeriod {
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
        for period in BudgetPeriod::all() {
            assert_eq!(&BudgetPeriod::from_str(period.as_str()).unwrap(), period);
        }
    }

    #[test]
    fn from_str_is_case_insensitive() {
        assert_eq!(BudgetPeriod::from_str("WEEKLY"), Ok(BudgetPeriod::Weekly));
        assert_eq!(BudgetPeriod::from_str("Yearly"), Ok(BudgetPeriod::Yearly));
    }

    #[test]
    fn from_str_rejects_unknown_values() {
        assert!(BudgetPeriod::from_str("bogus").is_err());
    }

    #[test]
    fn default_is_monthly() {
        assert_eq!(BudgetPeriod::default(), BudgetPeriod::Monthly);
    }

    #[test]
    fn mock_returns_a_valid_variant() {
        let period = BudgetPeriod::mock();
        assert!(BudgetPeriod::all().contains(&period));
    }

    #[test]
    fn monthly_bounds_cover_the_whole_calendar_month() {
        let today = NaiveDate::from_ymd_opt(2026, 2, 15).unwrap();
        let (start, end) = BudgetPeriod::Monthly.current_bounds(today);
        assert_eq!(start, NaiveDate::from_ymd_opt(2026, 2, 1).unwrap());
        assert_eq!(end, NaiveDate::from_ymd_opt(2026, 2, 28).unwrap());
    }

    #[test]
    fn monthly_bounds_handle_a_leap_february() {
        let today = NaiveDate::from_ymd_opt(2028, 2, 10).unwrap();
        let (_, end) = BudgetPeriod::Monthly.current_bounds(today);
        assert_eq!(end, NaiveDate::from_ymd_opt(2028, 2, 29).unwrap());
    }

    #[test]
    fn weekly_bounds_start_on_monday() {
        // 2026-02-18 is a Wednesday.
        let today = NaiveDate::from_ymd_opt(2026, 2, 18).unwrap();
        let (start, end) = BudgetPeriod::Weekly.current_bounds(today);
        assert_eq!(start.weekday(), chrono::Weekday::Mon);
        assert_eq!(start, NaiveDate::from_ymd_opt(2026, 2, 16).unwrap());
        assert_eq!(end, NaiveDate::from_ymd_opt(2026, 2, 22).unwrap());
    }

    #[test]
    fn quarterly_bounds_cover_a_calendar_quarter() {
        let today = NaiveDate::from_ymd_opt(2026, 5, 3).unwrap();
        let (start, end) = BudgetPeriod::Quarterly.current_bounds(today);
        assert_eq!(start, NaiveDate::from_ymd_opt(2026, 4, 1).unwrap());
        assert_eq!(end, NaiveDate::from_ymd_opt(2026, 6, 30).unwrap());
    }

    #[test]
    fn quarterly_bounds_handle_the_final_quarter_rolling_into_next_year() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 20).unwrap();
        let (start, end) = BudgetPeriod::Quarterly.current_bounds(today);
        assert_eq!(start, NaiveDate::from_ymd_opt(2026, 10, 1).unwrap());
        assert_eq!(end, NaiveDate::from_ymd_opt(2026, 12, 31).unwrap());
    }

    #[test]
    fn yearly_bounds_cover_the_whole_calendar_year() {
        let today = NaiveDate::from_ymd_opt(2026, 7, 4).unwrap();
        let (start, end) = BudgetPeriod::Yearly.current_bounds(today);
        assert_eq!(start, NaiveDate::from_ymd_opt(2026, 1, 1).unwrap());
        assert_eq!(end, NaiveDate::from_ymd_opt(2026, 12, 31).unwrap());
    }

    #[test]
    fn progress_fraction_is_zero_on_the_first_day_and_one_on_the_last() {
        let first = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();
        let last = NaiveDate::from_ymd_opt(2026, 3, 31).unwrap();
        assert_eq!(BudgetPeriod::Monthly.progress_fraction(first), 0.0);
        assert_eq!(BudgetPeriod::Monthly.progress_fraction(last), 1.0);
    }

    #[test]
    fn progress_fraction_is_roughly_halfway_mid_month() {
        // February 2026 has 28 days; day 15 (index 14) is roughly the midpoint.
        let mid = NaiveDate::from_ymd_opt(2026, 2, 15).unwrap();
        let fraction = BudgetPeriod::Monthly.progress_fraction(mid);
        assert!(fraction > 0.4 && fraction < 0.6, "fraction was {fraction}");
    }
}
