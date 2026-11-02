//! The Financial Year both Bills and Documents read, kept out of either domain so neither has to
//! reach into the other for it. Pure: no I/O and no UI.

use chrono::{Datelike, NaiveDate};

use crate::Period;

/// The month the Financial Year starts in: July, the Australian year, until Settings' "Financial
/// year starts" becomes a real Preference.
pub const FINANCIAL_YEAR_START_MONTH: u32 = 7;

/// The calendar year the Financial Year holding `date` starts in.
pub fn financial_year_start(date: NaiveDate) -> i32 {
    if date.month() >= FINANCIAL_YEAR_START_MONTH {
        date.year()
    } else {
        date.year() - 1
    }
}

/// The inclusive bounds of the Financial Year starting in `start_year`.
pub fn financial_year(start_year: i32) -> (NaiveDate, NaiveDate) {
    let first = Period {
        year: start_year,
        month: FINANCIAL_YEAR_START_MONTH,
    };
    let last = Period {
        year: start_year + 1,
        month: FINANCIAL_YEAR_START_MONTH,
    }
    .prev();
    (first.first_day(), last.last_day())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("a valid test date")
    }

    #[test]
    fn financial_years_start_in_july() {
        assert_eq!(financial_year_start(date(2026, 6, 30)), 2025);
        assert_eq!(financial_year_start(date(2026, 7, 1)), 2026);
        assert_eq!(financial_year(2025), (date(2025, 7, 1), date(2026, 6, 30)));
    }
}
