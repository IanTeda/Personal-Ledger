//! The Ledger's own Preferences (ADR-0061): its name, its Owner and the month its Financial Year
//! starts in, each checked once here so both Clients, the Settings page and the Create ledger
//! Dialog refuse the same values. Pure: no I/O and no UI.

use std::fmt;

use chrono::{Datelike, NaiveDate};

use crate::Period;

/// Why a text can't be a [`LedgerName`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LedgerNameError {
    /// Nothing is left once the text is trimmed.
    #[error("the ledger name is blank")]
    Blank,

    /// Trimmed, the text is over [`LedgerName::NAME_MAX`] characters.
    #[error("the ledger name is over {max} characters", max = LedgerName::NAME_MAX)]
    TooLong,
}

/// The name a Ledger shows in the Shell and on Settings › Ledger: trimmed, never blank and at
/// most [`NAME_MAX`](Self::NAME_MAX) characters (#1348).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LedgerName(String);

impl LedgerName {
    /// Longest Ledger name, counted in characters rather than bytes so a name in any script gets
    /// the same room.
    pub const NAME_MAX: usize = 60;

    /// A Ledger name from what the person typed, refused rather than cut so their text is kept
    /// for them to shorten.
    pub fn new(text: &str) -> Result<Self, LedgerNameError> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(LedgerNameError::Blank);
        }
        if trimmed.chars().count() > Self::NAME_MAX {
            return Err(LedgerNameError::TooLong);
        }
        Ok(Self(trimmed.to_string()))
    }

    /// The name a new Ledger starts with, from its file name without `.pldb`. A long stem is cut
    /// to [`NAME_MAX`](Self::NAME_MAX) characters so the default never starts invalid, and a blank
    /// one gives way to `fallback` (the Locale's `My ledger`), which is cut the same way. Only a
    /// blank `fallback` as well is refused.
    pub fn from_file_stem(stem: &str, fallback: &str) -> Result<Self, LedgerNameError> {
        Self::prefill(stem)
            .or_else(|| Self::prefill(fallback))
            .ok_or(LedgerNameError::Blank)
    }

    fn prefill(text: &str) -> Option<Self> {
        let cut: String = text.trim().chars().take(Self::NAME_MAX).collect();
        let cut = cut.trim_end();
        (!cut.is_empty()).then(|| Self(cut.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for LedgerName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Who the Ledger belongs to, trimmed. A blank Owner is no Owner, so it is never stored as `''`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Owner(String);

impl Owner {
    /// The Owner `text` names, or `None` when it is blank once trimmed: clearing the field
    /// clears the Owner rather than being refused.
    pub fn new(text: &str) -> Option<Self> {
        let trimmed = text.trim();
        (!trimmed.is_empty()).then(|| Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Owner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Why a number can't be a [`FinancialYearStart`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum FinancialYearStartError {
    /// The month is not 1–12.
    #[error("month {0} is not 1 to 12")]
    NotAMonth(u32),
}

/// The month a Ledger's Financial Year starts in. The year always starts on the 1st, so the
/// month is all there is to choose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FinancialYearStart(u32);

impl FinancialYearStart {
    /// The calendar year, and the United States' default.
    pub const JANUARY: Self = Self(1);

    /// The United Kingdom's year.
    pub const APRIL: Self = Self(4);

    /// The Australian year.
    pub const JULY: Self = Self(7);

    /// Every start a Ledger can choose, January first, as a select lists them.
    pub const ALL: [Self; 12] = [
        Self(1),
        Self(2),
        Self(3),
        Self(4),
        Self(5),
        Self(6),
        Self(7),
        Self(8),
        Self(9),
        Self(10),
        Self(11),
        Self(12),
    ];

    pub fn new(month: u32) -> Result<Self, FinancialYearStartError> {
        if (1..=12).contains(&month) {
            Ok(Self(month))
        } else {
            Err(FinancialYearStartError::NotAMonth(month))
        }
    }

    /// 1–12.
    pub fn month(self) -> u32 {
        self.0
    }

    /// The calendar year the Financial Year holding `date` starts in, which is how a year is
    /// keyed (#1342) whatever its label says.
    pub fn year_of(self, date: NaiveDate) -> i32 {
        if date.month() >= self.0 {
            date.year()
        } else {
            date.year() - 1
        }
    }

    /// The inclusive bounds of the Financial Year starting in `start_year`.
    pub fn bounds(self, start_year: i32) -> (NaiveDate, NaiveDate) {
        let first = Period {
            year: start_year,
            month: self.0,
        };
        let last = Period {
            year: start_year + 1,
            month: self.0,
        }
        .prev();
        (first.first_day(), last.last_day())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("a valid test date")
    }

    mod ledger_name {
        use super::*;

        #[test]
        fn a_name_is_trimmed() {
            let name = LedgerName::new("  Teda household \t").expect("a valid name");
            assert_eq!(name.as_str(), "Teda household");
            assert_eq!(name.to_string(), "Teda household");
        }

        #[test]
        fn a_blank_name_is_refused() {
            for text in ["", "   ", "\t\n"] {
                assert_eq!(
                    LedgerName::new(text),
                    Err(LedgerNameError::Blank),
                    "{text:?}"
                );
            }
        }

        #[test]
        fn sixty_characters_is_the_most_a_name_holds() {
            let at = "x".repeat(LedgerName::NAME_MAX);
            assert_eq!(LedgerName::NAME_MAX, 60);
            assert!(LedgerName::new(&at).is_ok());
            let over = "x".repeat(LedgerName::NAME_MAX + 1);
            assert_eq!(LedgerName::new(&over), Err(LedgerNameError::TooLong));
        }

        #[test]
        fn the_limit_counts_characters_after_trimming() {
            // 60 two-byte characters are 120 bytes but still fit.
            let accented = "é".repeat(60);
            assert!(LedgerName::new(&format!("  {accented}  ")).is_ok());
            assert_eq!(
                LedgerName::new(&"é".repeat(61)),
                Err(LedgerNameError::TooLong)
            );
        }

        #[test]
        fn a_file_stem_prefills_the_name() {
            let name = LedgerName::from_file_stem("household", "My ledger").expect("a name");
            assert_eq!(name.as_str(), "household");
        }

        #[test]
        fn a_long_stem_is_cut_to_sixty_characters() {
            let stem = "a".repeat(75);
            let name = LedgerName::from_file_stem(&stem, "My ledger").expect("a name");
            assert_eq!(name.as_str(), "a".repeat(60));
        }

        #[test]
        fn a_cut_stem_loses_its_trailing_whitespace() {
            let stem = format!("{}   tail", "b".repeat(58));
            let name = LedgerName::from_file_stem(&stem, "My ledger").expect("a name");
            assert_eq!(name.as_str(), "b".repeat(58));
            assert!(LedgerName::new(name.as_str()).is_ok());
        }

        #[test]
        fn a_blank_stem_falls_back() {
            for stem in ["", "  "] {
                let name = LedgerName::from_file_stem(stem, "My ledger").expect("a name");
                assert_eq!(name.as_str(), "My ledger", "{stem:?}");
            }
        }

        #[test]
        fn a_blank_fallback_too_is_refused() {
            assert_eq!(
                LedgerName::from_file_stem(" ", " "),
                Err(LedgerNameError::Blank)
            );
        }
    }

    mod owner {
        use super::*;

        #[test]
        fn an_owner_is_trimmed() {
            let owner = Owner::new("  Ian Teda ").expect("an Owner");
            assert_eq!(owner.as_str(), "Ian Teda");
            assert_eq!(owner.to_string(), "Ian Teda");
        }

        #[test]
        fn a_blank_owner_is_no_owner() {
            for text in ["", "  ", "\t"] {
                assert_eq!(Owner::new(text), None, "{text:?}");
            }
        }
    }

    mod financial_year_start {
        use super::*;

        #[test]
        fn a_start_is_a_month() {
            assert_eq!(
                FinancialYearStart::new(1).map(FinancialYearStart::month),
                Ok(1)
            );
            assert_eq!(
                FinancialYearStart::new(12).map(FinancialYearStart::month),
                Ok(12)
            );
            assert_eq!(FinancialYearStart::new(7), Ok(FinancialYearStart::JULY));
            for month in [0, 13, 99] {
                assert_eq!(
                    FinancialYearStart::new(month),
                    Err(FinancialYearStartError::NotAMonth(month))
                );
            }
        }

        #[test]
        fn all_runs_january_to_december() {
            let months: Vec<u32> = FinancialYearStart::ALL
                .iter()
                .map(|start| start.month())
                .collect();
            assert_eq!(months, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
        }

        #[test]
        fn a_july_year_crosses_the_calendar_year() {
            let july = FinancialYearStart::JULY;
            assert_eq!(july.year_of(date(2026, 6, 30)), 2025);
            assert_eq!(july.year_of(date(2026, 7, 1)), 2026);
            assert_eq!(july.year_of(date(2026, 12, 31)), 2026);
            assert_eq!(july.year_of(date(2027, 1, 1)), 2026);
            assert_eq!(july.bounds(2025), (date(2025, 7, 1), date(2026, 6, 30)));
        }

        #[test]
        fn a_january_year_is_the_calendar_year() {
            let january = FinancialYearStart::JANUARY;
            assert_eq!(january.year_of(date(2025, 12, 31)), 2025);
            assert_eq!(january.year_of(date(2026, 1, 1)), 2026);
            assert_eq!(january.bounds(2026), (date(2026, 1, 1), date(2026, 12, 31)));
        }

        #[test]
        fn an_april_year_crosses_the_calendar_year() {
            let april = FinancialYearStart::APRIL;
            assert_eq!(april.year_of(date(2026, 3, 31)), 2025);
            assert_eq!(april.year_of(date(2026, 4, 1)), 2026);
            assert_eq!(april.year_of(date(2027, 1, 15)), 2026);
            assert_eq!(april.bounds(2025), (date(2025, 4, 1), date(2026, 3, 31)));
        }

        #[test]
        fn a_march_year_ends_on_a_leap_day() {
            let march = FinancialYearStart::new(3).expect("March is a month");
            assert_eq!(march.bounds(2027), (date(2027, 3, 1), date(2028, 2, 29)));
        }
    }
}
