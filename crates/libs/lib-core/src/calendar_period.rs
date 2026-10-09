//! The calendar month both Bills and Budgets page through, kept out of either domain so neither
//! has to reach into the other for it. Pure: no I/O and no UI.

use chrono::{Datelike, NaiveDate};

/// A calendar month: the period the Bills Schedule tab and the Budgets surface page through.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Period {
    pub year: i32,
    /// 1–12.
    pub month: u32,
}

impl Period {
    pub fn of(date: NaiveDate) -> Self {
        Self {
            year: date.year(),
            month: date.month(),
        }
    }

    pub fn first_day(self) -> NaiveDate {
        NaiveDate::from_ymd_opt(self.year, self.month, 1).unwrap_or(NaiveDate::MIN)
    }

    pub fn last_day(self) -> NaiveDate {
        self.next().first_day().pred_opt().unwrap_or(NaiveDate::MAX)
    }

    pub fn contains(self, date: NaiveDate) -> bool {
        Self::of(date) == self
    }

    pub fn next(self) -> Self {
        self.shift(1)
    }

    pub fn prev(self) -> Self {
        self.shift(-1)
    }

    pub fn shift(self, months: i32) -> Self {
        let index = self.year * 12 + self.month as i32 - 1 + months;
        Self {
            year: index.div_euclid(12),
            month: index.rem_euclid(12) as u32 + 1,
        }
    }
}
