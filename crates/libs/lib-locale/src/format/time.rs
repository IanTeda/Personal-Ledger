//! Locale-aware time-of-day formatting.

use chrono::{NaiveTime, Timelike};
use icu_datetime::NoCalendarFormatter;
use icu_datetime::fieldsets::T;
use icu_datetime::input::Time;

use super::{Cache, cached, icu_locale};
use crate::error::{Error, Result};
use crate::locale::Locale;

thread_local! {
    static SECONDS: Cache<Locale, NoCalendarFormatter<T>> = Cache::default();
}

fn try_format(time: NaiveTime) -> Result<String> {
    let locale = crate::locale();
    let formatter = cached(&SECONDS, locale, || {
        NoCalendarFormatter::try_new(icu_locale(locale)?.into(), T::hms())
            .map_err(|error| Error::Format(format!("time formatter for {locale}: {error}")))
    })?;
    // chrono's leap second (second 59, nanosecond >= 1e9) has no ICU equivalent, and the
    // formatter shows no fraction, so the nanoseconds are dropped rather than clamped.
    let field = |value: u32| {
        u8::try_from(value).map_err(|_| Error::Format(format!("time field out of range: {value}")))
    };
    let icu = Time::try_new(
        field(time.hour())?,
        field(time.minute())?,
        field(time.second())?,
        0,
    )
    .map_err(|error| Error::Format(format!("time {time} is not representable: {error}")))?;
    Ok(formatter.format(&icu).to_string())
}

/// A time of day to the second in the Locale's own clock (`3:04:05 pm` in `en-AU`, `15:04:05` in
/// `en-GB`), e.g. when a Toast was last raised. Falls back to 24-hour `HH:MM:SS`.
pub fn format_time(time: NaiveTime) -> String {
    try_format(time).unwrap_or_else(|error| {
        tracing::warn!(%error, "time formatting failed; using 24-hour");
        time.format("%H:%M:%S").to_string()
    })
}
