//! Typed date input: the Locale's numeric short form, ISO, and relative words.
//!
//! ICU4X formats dates but does not parse them, so this is a small parser over
//! `NaiveDate::from_ymd_opt`. The field order and separator are read from the Locale's resolved
//! short pattern (`d/M/yy` in `en-AU`, `M/d/yy` in `en-US`, `dd/MM/y` in `en-GB`), so it accepts
//! exactly what the formatter prints and a new Locale needs no table.
//!
//! The Locale is the only thing that decides day versus month: `03/09/2026` means one date per
//! Locale, and `13/09` in a month-first Locale is an error, never silently reordered. Years must
//! have four digits.

use chrono::{Datelike, Duration, NaiveDate};
use lib_core::DateStyle;

use super::date::short_pattern;
use super::{Cache, cached, format_date};
use crate::locale::Locale;

thread_local! {
    static SHAPES: Cache<Locale, Option<Shape>> = Cache::default();
}

/// The part of a date a validation error points at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateField {
    /// The day of the month.
    Day,

    /// The month.
    Month,

    /// The year.
    Year,
}

/// Why typed input was not a date. The English `Display` is for developers and logs; a bin shows
/// [`date_error_message`] to the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DateInputError {
    /// Nothing was typed. Callers with an optional field treat this as "no value".
    #[error("no date was entered")]
    Empty,

    /// The text is not in the Locale's short form, ISO, or a known word.
    #[error("the text is not a date in the Locale's form, ISO, or a known word")]
    Unrecognised,

    /// The text had the right shape but the field is not valid for that date (month 13, or
    /// day 31 in September), which is what a day and month mix-up produces.
    #[error("the {0:?} is not valid for a date")]
    OutOfRange(DateField),

    /// A year that is not four digits.
    #[error("a year must have four digits")]
    BadYear,

    /// Only a day and month were typed, at a call site that requires the year.
    #[error("the year is required")]
    YearRequired,
}

/// How a call site wants typed dates read.
#[derive(Debug, Clone, Copy, Default)]
pub struct DateInputOptions {
    /// The user's date style Preference. `Some(DateStyle::Iso)` makes ISO the only numeric form;
    /// anything else reads the Locale's short form (and ISO).
    pub style: Option<DateStyle>,

    /// Accept a day and month with no year, taking today's year. For filter fields; an entry
    /// form should leave this off, because a silently wrong year is worse than a rejection.
    pub allow_yearless: bool,
}

/// Field order and separator read from a resolved short pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Shape {
    order: [DateField; 3],
    separator: char,
}

impl Shape {
    /// Reads `d/M/yy`-style UTS 35 patterns. Month names (`MMM`), any other letter, a repeated field or a
    /// missing separator gives `None`, so the Locale falls back to ISO and words rather than mis-parse.
    fn from_pattern(pattern: &str) -> Option<Shape> {
        if pattern.contains("MMM") {
            return None;
        }
        let mut order: Vec<DateField> = Vec::new();
        let mut separator = None;
        let mut previous = None;
        for ch in pattern.chars() {
            let field = match ch {
                'd' => Some(DateField::Day),
                'M' => Some(DateField::Month),
                'y' => Some(DateField::Year),
                other if other.is_alphabetic() => return None,
                other => {
                    separator = separator.or(Some(other));
                    None
                }
            };
            if field.is_some() && field != previous {
                order.push(field?);
            }
            previous = field;
        }
        let order: [DateField; 3] = order.try_into().ok()?;
        let unique = order[0] != order[1] && order[1] != order[2] && order[0] != order[2];
        unique.then_some(Shape {
            order,
            separator: separator?,
        })
    }
}

impl Shape {
    /// Today's date in this shape, with a four-digit year (the form typed input accepts).
    fn example(&self, today: NaiveDate) -> String {
        let parts: Vec<String> = self
            .order
            .iter()
            .map(|field| match field {
                DateField::Day => today.day().to_string(),
                DateField::Month => today.month().to_string(),
                DateField::Year => today.year().to_string(),
            })
            .collect();
        parts.join(&self.separator.to_string())
    }
}

fn shape(locale: Locale) -> Option<Shape> {
    let cached = cached(&SHAPES, locale, || {
        Ok::<_, crate::error::Error>(
            short_pattern(locale)
                .ok()
                .and_then(|p| Shape::from_pattern(&p)),
        )
    })
    .ok()?;
    *cached
}

/// The accepted relative words and their offset in days from today: the built-in English table
/// unioned with the Locale's `date-word-*` Messages, so English always works.
fn words() -> Vec<(String, i64)> {
    let locale = crate::locale().formatting_locale();
    let (today, yesterday, tomorrow) = crate::with_locale(locale, || {
        (
            crate::msg::date_word_today(),
            crate::msg::date_word_yesterday(),
            crate::msg::date_word_tomorrow(),
        )
    });
    let mut words: Vec<(String, i64)> = Vec::new();
    for (word, offset) in [
        (today, 0),
        (yesterday, -1),
        (tomorrow, 1),
        ("today".to_string(), 0),
        ("yesterday".to_string(), -1),
        ("tomorrow".to_string(), 1),
    ] {
        let word = word.trim().to_lowercase();
        if !words.iter().any(|(existing, _)| *existing == word) {
            words.push((word, offset));
        }
    }
    words
}

/// Reads typed input with the default options: the Locale's short form, ISO and words, with a
/// required year.
pub fn parse_date(text: &str, today: NaiveDate) -> std::result::Result<NaiveDate, DateInputError> {
    parse_date_with(text, today, &DateInputOptions::default())
}

/// Reads typed input as a date for the Locale in effect. `today` is injected so tests are
/// deterministic and the time zone stays the caller's decision.
pub fn parse_date_with(
    text: &str,
    today: NaiveDate,
    options: &DateInputOptions,
) -> std::result::Result<NaiveDate, DateInputError> {
    let shape = if options.style == Some(DateStyle::Iso) {
        None
    } else {
        shape(crate::locale())
    };
    parse_with(text, today, options.allow_yearless, shape, &words())
}

fn parse_with(
    text: &str,
    today: NaiveDate,
    allow_yearless: bool,
    shape: Option<Shape>,
    words: &[(String, i64)],
) -> std::result::Result<NaiveDate, DateInputError> {
    let text = text.trim().to_lowercase();
    if text.is_empty() {
        return Err(DateInputError::Empty);
    }

    if let Some((_, offset)) = words.iter().find(|(word, _)| *word == text) {
        return today
            .checked_add_signed(Duration::days(*offset))
            .ok_or(DateInputError::Unrecognised);
    }

    let iso_parts: Vec<&str> = text.split('-').collect();
    if let [year, month, day] = iso_parts[..]
        && year.len() == 4
        && all_digits(year)
        && short_digits(month)
        && short_digits(day)
    {
        return build(number(year), number(month), number(day));
    }

    let Some(shape) = shape else {
        return Err(DateInputError::Unrecognised);
    };
    let parts: Vec<&str> = text.split(shape.separator).collect();
    if !parts.iter().all(|part| all_digits(part)) {
        return Err(DateInputError::Unrecognised);
    }

    match parts[..] {
        [a, b, c] => {
            let (mut year, mut month, mut day) = (None, None, None);
            for (field, part) in shape.order.iter().zip([a, b, c]) {
                match field {
                    DateField::Year => year = Some(part),
                    DateField::Month => month = Some(part),
                    DateField::Day => day = Some(part),
                }
            }
            let (Some(year), Some(month), Some(day)) = (year, month, day) else {
                return Err(DateInputError::Unrecognised);
            };
            if !short_digits(month) || !short_digits(day) {
                return Err(DateInputError::Unrecognised);
            }
            build(expand_year(year)?, number(month), number(day))
        }
        [a, b] => {
            if !short_digits(a) || !short_digits(b) {
                return Err(DateInputError::Unrecognised);
            }
            if !allow_yearless {
                return Err(DateInputError::YearRequired);
            }
            let (mut month, mut day) = (None, None);
            for (field, part) in shape
                .order
                .iter()
                .filter(|f| **f != DateField::Year)
                .zip([a, b])
            {
                match field {
                    DateField::Month => month = Some(part),
                    _ => day = Some(part),
                }
            }
            let (Some(month), Some(day)) = (month, day) else {
                return Err(DateInputError::Unrecognised);
            };
            build(i64::from(today.year()), number(month), number(day))
        }
        _ => Err(DateInputError::Unrecognised),
    }
}

fn all_digits(part: &str) -> bool {
    !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit())
}

/// One or two ASCII digits, for a day or month.
fn short_digits(part: &str) -> bool {
    all_digits(part) && part.len() <= 2
}

/// Parses ASCII digits already checked by [`all_digits`] and short enough not to overflow.
fn number(part: &str) -> i64 {
    part.bytes()
        .fold(0, |total, digit| total * 10 + i64::from(digit - b'0'))
}

/// Years must be four digits: a two-digit year is ambiguous about the century, and a ledger
/// should reject it rather than guess.
fn expand_year(part: &str) -> std::result::Result<i64, DateInputError> {
    if part.len() == 4 {
        Ok(number(part))
    } else {
        Err(DateInputError::BadYear)
    }
}

fn build(year: i64, month: i64, day: i64) -> std::result::Result<NaiveDate, DateInputError> {
    let year = i32::try_from(year).map_err(|_| DateInputError::OutOfRange(DateField::Year))?;
    let month = u32::try_from(month).ok().filter(|m| (1..=12).contains(m));
    let Some(month) = month else {
        return Err(DateInputError::OutOfRange(DateField::Month));
    };
    u32::try_from(day)
        .ok()
        .and_then(|day| NaiveDate::from_ymd_opt(year, month, day))
        .ok_or(DateInputError::OutOfRange(DateField::Day))
}

/// A date as typed input reads it: ISO when the style is `Iso`, else the Locale's short field
/// order with a four-digit year (`3/9/2026` in `en-AU`). Use this to fill a text field the user
/// will edit, since [`parse_date`] reads it back, which the display styles do not promise.
pub fn format_date_input(date: NaiveDate, style: Option<DateStyle>) -> String {
    match (style, shape(crate::locale())) {
        (Some(DateStyle::Iso), _) | (_, None) => format_date(date, Some(DateStyle::Iso)),
        (_, Some(shape)) => shape.example(date),
    }
}

/// The user-facing text for a typed-date error, as a Message with the expected format as an
/// argument (today's date in the Locale's short form, its ISO form and the accepted words),
/// never a literal `YYYY-MM-DD`.
pub fn date_error_message(
    error: &DateInputError,
    today: NaiveDate,
    options: &DateInputOptions,
) -> String {
    match error {
        DateInputError::Empty => crate::msg::date_error_empty(),
        DateInputError::BadYear => crate::msg::date_error_bad_year(),
        DateInputError::YearRequired => crate::msg::date_error_year_required(),
        DateInputError::OutOfRange(field) => {
            let field = match field {
                DateField::Day => crate::msg::date_field_day(),
                DateField::Month => crate::msg::date_field_month(),
                DateField::Year => crate::msg::date_field_year(),
            };
            crate::msg::date_error_out_of_range(&field)
        }
        DateInputError::Unrecognised => {
            let iso = format_date(today, Some(DateStyle::Iso));
            let words = words()
                .into_iter()
                .map(|(word, _)| word)
                .collect::<Vec<_>>()
                .join(", ");
            match (options.style, shape(crate::locale())) {
                (Some(DateStyle::Iso), _) | (_, None) => {
                    crate::msg::date_error_unrecognised_iso(&iso, &words)
                }
                (_, Some(shape)) => {
                    crate::msg::date_error_unrecognised(&shape.example(today), &iso, &words)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn day(year: i32, month: u32, date: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, date).expect("test date is valid")
    }

    #[test]
    fn reads_shapes_from_resolved_patterns() {
        let au = Shape::from_pattern("d/M/yy").expect("en-AU pattern");
        assert_eq!(
            au.order,
            [DateField::Day, DateField::Month, DateField::Year]
        );
        assert_eq!(au.separator, '/');
        let us = Shape::from_pattern("M/d/yy").expect("en-US pattern");
        assert_eq!(
            us.order,
            [DateField::Month, DateField::Day, DateField::Year]
        );
        let gb = Shape::from_pattern("dd/MM/y").expect("en-GB pattern");
        assert_eq!(
            gb.order,
            [DateField::Day, DateField::Month, DateField::Year]
        );
    }

    #[test]
    fn unusual_patterns_give_no_shape() {
        assert_eq!(Shape::from_pattern("d MMM y"), None);
        assert_eq!(Shape::from_pattern("d/M"), None);
        assert_eq!(Shape::from_pattern("dMy"), None);
        assert_eq!(Shape::from_pattern("dd/dd/y"), None);
    }

    #[test]
    fn relative_words_union_message_words_with_english() {
        let words = [("hoy".to_string(), 0), ("today".to_string(), 0)];
        let today = day(2026, 9, 20);
        assert_eq!(parse_with("hoy", today, false, None, &words), Ok(today));
        assert_eq!(parse_with(" TODAY ", today, false, None, &words), Ok(today));
    }

    #[test]
    fn years_must_have_four_digits() {
        assert_eq!(expand_year("2026"), Ok(2026));
        for part in ["26", "1", "202", "20260"] {
            assert_eq!(expand_year(part), Err(DateInputError::BadYear), "{part}");
        }
    }

    #[test]
    fn overflowing_years_are_rejected_not_wrapped() {
        assert_eq!(
            build(3_000_000_000, 1, 1),
            Err(DateInputError::OutOfRange(DateField::Year))
        );
    }
}
