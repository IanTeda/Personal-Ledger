//! The one place the TUI formats amounts and dates, through `lib-locale`, so grouping, decimal
//! marks, month names and their order follow the Locale in effect. Views keep only their own
//! policy (how many decimal places a column shows) and call these.
//!
//! The sign of an amount (its minus, and that a zero is never negative) is `lib-locale`'s; this
//! module only picks the places. Dates are lower case, as the views set them.
//! Anything wider than a fixed column must be measured with `UnicodeWidthStr::width`, never
//! `str::len()`, since a Locale's month names vary in length.

use bigdecimal::{BigDecimal, FromPrimitive};
use chrono::{Datelike, NaiveDate};
use lib_core::Money;
use lib_locale::format::{
    AmountStyle, format_amount, format_date, format_month_day, format_year_month,
};

/// An amount at exactly `decimal_places` (a Unit's own precision), grouped, marked and signed by
/// the Locale.
pub fn money(value: &Money, decimal_places: i64) -> String {
    let places = u32::try_from(decimal_places.max(0)).unwrap_or(0);
    format_amount(value, AmountStyle::fixed(places)).text
}

/// An amount that keeps its cents only when it has a fractional part (`142 100` but `12 480.40`),
/// for a summary where full precision matters.
pub fn money_natural(value: &Money) -> String {
    format_amount(value, AmountStyle::natural()).text
}

/// An amount as `f64`, for chart data and fixture arithmetic only: chart data is approximate
/// already, but ledger amounts stay [`Money`] and are formatted with [`money`].
pub fn money_to_f64(value: &Money) -> f64 {
    value.0.to_string().parse().unwrap_or(0.0)
}

/// A chart or fixture figure held as `f64`, at `decimal_places`. Chart data is approximate
/// already; ledger amounts stay [`Money`] and use [`money`].
pub fn money_f64(amount: f64, decimal_places: i64) -> String {
    money(
        &Money(BigDecimal::from_f64(amount).unwrap_or_default()),
        decimal_places,
    )
}

/// A whole count, grouped by the Locale (`1 284`), for a Message that names a number of rows, days
/// or months. Counts a Message pluralises on are passed as `i64` instead and formatted by Fluent.
pub fn count(value: i64) -> String {
    money(&Money(BigDecimal::from(value)), 0)
}

/// A full date in the Locale's medium form, lower case (`12 sept 2026`).
pub fn date(date: NaiveDate) -> String {
    format_date(date, None).to_lowercase()
}

/// A day and month for a narrow column, lower case (`12 sept`, `sep 12` in `en-US`).
pub fn day_month(date: NaiveDate) -> String {
    format_month_day(date).to_lowercase()
}

/// A month and year for a chart axis, lower case (`sept 2026`).
pub fn month_year(date: NaiveDate) -> String {
    format_year_month(date.year(), date.month()).to_lowercase()
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use lib_locale::{Locale, with_locale};

    use super::*;

    fn amount(text: &str) -> Money {
        Money::from_str(text).expect("test amount parses")
    }

    fn day(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).expect("test date is valid")
    }

    #[test]
    fn money_groups_at_the_units_own_precision() {
        for locale in [Locale::EnUs, Locale::EnGb, Locale::EnAu] {
            with_locale(locale, || {
                assert_eq!(money(&amount("1284.3"), 2), "1,284.30");
                assert_eq!(money(&amount("412.48"), 4), "412.4800");
                assert_eq!(money(&amount("0.184"), 8), "0.18400000");
                assert_eq!(money(&amount("142100"), 0), "142,100");
            });
        }
    }

    #[test]
    fn natural_money_keeps_cents_only_when_there_are_some() {
        with_locale(Locale::EnAu, || {
            assert_eq!(money_natural(&amount("142100")), "142,100");
            assert_eq!(money_natural(&amount("12480.4")), "12,480.40");
        });
    }

    #[test]
    fn chart_figures_go_through_the_same_formatter() {
        with_locale(Locale::EnAu, || {
            assert_eq!(money_f64(40_637.79, 2), "40,637.79");
            assert_eq!(money_f64(1200.0, 0), "1,200");
            assert_eq!(money_f64(3.5, 1), "3.5");
        });
    }

    #[test]
    fn dates_are_lower_case_and_follow_the_locale() {
        let d = day(2026, 9, 12);
        with_locale(Locale::EnAu, || {
            assert_eq!(date(d), "12 sept 2026");
            assert_eq!(day_month(d), "12 sept");
            assert_eq!(month_year(d), "sept 2026");
        });
        with_locale(Locale::EnUs, || {
            assert_eq!(date(d), "sep 12, 2026");
            assert_eq!(day_month(d), "sep 12");
        });
    }
}
