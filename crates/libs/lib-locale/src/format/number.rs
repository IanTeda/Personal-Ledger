//! Locale-aware decimal formatting and the exact `Money` to ICU4X decimal bridge.

use std::str::FromStr;

use fixed_decimal::{Decimal, Sign, SignedRoundingMode, UnsignedRoundingMode};
use icu_decimal::DecimalFormatter;
use lib_core::Money;

use std::rc::Rc;

use super::{Cache, cached, icu_locale};
use crate::error::{Error, Result};
use crate::locale::Locale;

thread_local! {
    static FORMATTERS: Cache<Locale, DecimalFormatter> = Cache::default();
}

/// Converts an exact amount to ICU4X's decimal, rounded half away from zero (as a ledger expects) to
/// `fraction_digits` places and padded so `12.5` at two places shows `12.50`.
///
/// Uses `to_plain_string()`: `Display` can emit scientific notation ICU4X cannot parse.
pub(super) fn to_decimal(amount: &Money, fraction_digits: u32) -> Result<Decimal> {
    let plain = amount.0.to_plain_string();
    let mut decimal = Decimal::from_str(&plain)
        .map_err(|error| Error::Format(format!("cannot convert {plain} to a decimal: {error}")))?;
    let position = -i16::try_from(fraction_digits)
        .map_err(|_| Error::Format(format!("too many fraction digits: {fraction_digits}")))?;
    decimal.round_with_mode(
        position,
        SignedRoundingMode::Unsigned(UnsignedRoundingMode::HalfExpand),
    );
    decimal.pad_end(position);
    if decimal.is_zero() {
        // Rounding `-0.004` must not show as `-0.00`.
        decimal.set_sign(Sign::None);
    }
    Ok(decimal)
}

pub(super) fn decimal_formatter(locale: Locale) -> Result<Rc<DecimalFormatter>> {
    cached(&FORMATTERS, locale, || {
        let prefs = (&icu_locale(locale)?).into();
        DecimalFormatter::try_new(prefs, Default::default())
            .map_err(|error| Error::Format(format!("decimal formatter for {locale}: {error}")))
    })
}

fn try_format(amount: &Money, fraction_digits: u32) -> Result<String> {
    let decimal = to_decimal(amount, fraction_digits)?;
    let formatter = decimal_formatter(crate::locale())?;
    Ok(formatter.format(&decimal).to_string())
}

/// Formats a quantity with the Locale's grouping and decimal separators, rounded to
/// `fraction_digits` places (`1,234.50` in `en-AU`).
pub fn format_number(amount: &Money, fraction_digits: u32) -> String {
    try_format(amount, fraction_digits).unwrap_or_else(|error| {
        tracing::warn!(%error, "number formatting failed; using plain text");
        amount.0.to_plain_string()
    })
}
