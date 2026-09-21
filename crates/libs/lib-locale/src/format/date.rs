//! Locale-aware date formatting.

use chrono::{Datelike, NaiveDate};
use icu_calendar::Date;
use icu_datetime::DateTimeFormatter;
use icu_datetime::fieldsets::{M, MD, YM, YMD};
use lib_core::DateStyle;

use std::rc::Rc;

use super::{Cache, cached, icu_locale};
use crate::error::{Error, Result};
use crate::locale::Locale;

thread_local! {
    static FORMATTERS: Cache<(Locale, u8), DateTimeFormatter<YMD>> = Cache::default();
    static YEAR_MONTH: Cache<Locale, DateTimeFormatter<YM>> = Cache::default();
    static MONTH: Cache<Locale, DateTimeFormatter<M>> = Cache::default();
    static MONTH_DAY: Cache<Locale, DateTimeFormatter<MD>> = Cache::default();
}

/// `DateStyle` is not `Hash`, and `Iso` never reaches a formatter.
fn style_key(style: DateStyle) -> u8 {
    match style {
        DateStyle::Short => 0,
        DateStyle::Medium | DateStyle::Iso => 1,
        DateStyle::Long => 2,
    }
}

fn formatter(locale: Locale, style: DateStyle) -> Result<Rc<DateTimeFormatter<YMD>>> {
    cached(&FORMATTERS, (locale, style_key(style)), || {
        let prefs = icu_locale(locale)?.into();
        let field_set = match style {
            DateStyle::Short => YMD::short(),
            DateStyle::Long => YMD::long(),
            DateStyle::Medium | DateStyle::Iso => YMD::medium(),
        };
        DateTimeFormatter::try_new(prefs, field_set)
            .map_err(|error| Error::Format(format!("date formatter for {locale}: {error}")))
    })
}

fn try_format(date: NaiveDate, style: DateStyle) -> Result<String> {
    let month = u8::try_from(date.month())
        .map_err(|_| Error::Format(format!("month out of range: {}", date.month())))?;
    let day = u8::try_from(date.day())
        .map_err(|_| Error::Format(format!("day out of range: {}", date.day())))?;
    let iso = Date::try_new_iso(date.year(), month, day)
        .map_err(|error| Error::Format(format!("date {date} is not representable: {error}")))?;
    Ok(formatter(crate::locale(), style)?.format(&iso).to_string())
}

/// The Locale's resolved short pattern (`d/M/yy` in `en-AU`), for the typed-input parser to
/// read the field order and separator from.
pub(super) fn short_pattern(locale: Locale) -> Result<String> {
    let sample = Date::try_new_iso(2026, 9, 3)
        .map_err(|error| Error::Format(format!("sample date rejected: {error}")))?;
    let formatter = formatter(locale, DateStyle::Short)?;
    Ok(formatter.format(&sample).pattern().to_string())
}

/// Formats a date in the given style, or in the Locale's default (medium, e.g. `3 Sept 2026` in
/// `en-AU`) when `style` is `None`. `Iso` always renders `YYYY-MM-DD`, overriding the Locale.
pub fn format_date(date: NaiveDate, style: Option<DateStyle>) -> String {
    let style = style.unwrap_or(DateStyle::Medium);
    if style == DateStyle::Iso {
        return date.format("%Y-%m-%d").to_string();
    }
    try_format(date, style).unwrap_or_else(|error| {
        tracing::warn!(%error, "date formatting failed; using ISO");
        date.format("%Y-%m-%d").to_string()
    })
}

fn first_of_month(year: i32, month: u32) -> Result<Date<icu_calendar::Iso>> {
    let month =
        u8::try_from(month).map_err(|_| Error::Format(format!("month out of range: {month}")))?;
    Date::try_new_iso(year, month, 1)
        .map_err(|error| Error::Format(format!("{year}-{month} is not representable: {error}")))
}

fn try_year_month(year: i32, month: u32) -> Result<String> {
    let locale = crate::locale();
    let formatter = cached(&YEAR_MONTH, locale, || {
        DateTimeFormatter::try_new(icu_locale(locale)?.into(), YM::medium())
            .map_err(|error| Error::Format(format!("year-month formatter for {locale}: {error}")))
    })?;
    Ok(formatter.format(&first_of_month(year, month)?).to_string())
}

fn try_month(month: u32) -> Result<String> {
    let locale = crate::locale();
    let formatter = cached(&MONTH, locale, || {
        DateTimeFormatter::try_new(icu_locale(locale)?.into(), M::medium())
            .map_err(|error| Error::Format(format!("month formatter for {locale}: {error}")))
    })?;
    Ok(formatter.format(&first_of_month(2026, month)?).to_string())
}

/// A month and year for a chart axis or heading, abbreviated by the Locale (`Mar 2025`). Falls back
/// to `2025-03` if the month is not a real one.
pub fn format_year_month(year: i32, month: u32) -> String {
    try_year_month(year, month).unwrap_or_else(|error| {
        tracing::warn!(%error, "year-month formatting failed; using ISO");
        format!("{year:04}-{month:02}")
    })
}

/// A month's abbreviated name in the Locale (`Sept` in `en-AU`), for a chart axis. Falls back to
/// the month number if it is not a real month.
pub fn format_month(month: u32) -> String {
    try_month(month).unwrap_or_else(|error| {
        tracing::warn!(%error, "month formatting failed; using the number");
        month.to_string()
    })
}

fn try_month_day(date: NaiveDate) -> Result<String> {
    let locale = crate::locale();
    let formatter = cached(&MONTH_DAY, locale, || {
        DateTimeFormatter::try_new(icu_locale(locale)?.into(), MD::medium())
            .map_err(|error| Error::Format(format!("month-day formatter for {locale}: {error}")))
    })?;
    let month = u8::try_from(date.month())
        .map_err(|_| Error::Format(format!("month out of range: {}", date.month())))?;
    let day = u8::try_from(date.day())
        .map_err(|_| Error::Format(format!("day out of range: {}", date.day())))?;
    let iso = Date::try_new_iso(date.year(), month, day)
        .map_err(|error| Error::Format(format!("date {date} is not representable: {error}")))?;
    Ok(formatter.format(&iso).to_string())
}

/// A day and month without the year, abbreviated by the Locale (`12 Sept` in `en-AU`, `Sep 12` in
/// `en-US`), for a narrow column. Falls back to ISO if the date cannot be formatted.
pub fn format_month_day(date: NaiveDate) -> String {
    try_month_day(date).unwrap_or_else(|error| {
        tracing::warn!(%error, "month-day formatting failed; using ISO");
        date.format("%Y-%m-%d").to_string()
    })
}
