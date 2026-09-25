//! Signed ledger amounts: the Locale's grouping and decimal marks with a real minus.
//!
//! The sign is a presentation decision owned here, not by the bins: a negative shows U+2212
//! (never the hyphen ICU4X's `en` data uses), a value that rounds to zero is never negative, and
//! a positive can carry an explicit `+`. Negativity is read from the rounded ICU4X decimal, never
//! by re-parsing formatted text.

use fixed_decimal::Sign;
use lib_core::Money;

use super::number::{decimal_formatter, to_decimal};
use crate::error::Result;

/// The minus shown before a negative amount: U+2212, not a hyphen. The one place it is spelt.
const MINUS: char = '\u{2212}';

/// How many fractional digits an amount is shown with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Places {
    /// Exactly this many places, rounded half away from zero (a Unit's own precision).
    Fixed(u32),

    /// The amount's own scale, as entered: a fund's `0.4120` keeps four places, `1240` none.
    Own,

    /// Cents only when there are some: `142,100` but `12,480.40`.
    Natural,
}

/// How [`format_amount`] shows an amount: its places, and whether a positive shows `+`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmountStyle {
    /// The fractional digits shown.
    pub places: Places,

    /// Whether a positive, non-zero amount is marked with `+` (a "signed" column).
    pub plus: bool,
}

impl AmountStyle {
    /// Exactly `places` fractional digits, no `+`.
    pub const fn fixed(places: u32) -> Self {
        AmountStyle {
            places: Places::Fixed(places),
            plus: false,
        }
    }

    /// The amount's own scale, no `+`.
    pub const fn own() -> Self {
        AmountStyle {
            places: Places::Own,
            plus: false,
        }
    }

    /// Cents only when there are some, no `+`.
    pub const fn natural() -> Self {
        AmountStyle {
            places: Places::Natural,
            plus: false,
        }
    }

    /// This style with a `+` in front of a positive, non-zero amount.
    pub const fn with_plus(self) -> Self {
        AmountStyle { plus: true, ..self }
    }
}

/// A formatted amount and whether it is negative, so a caller can mark a negative by more than
/// colour alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Amount {
    /// The text to show (`−1,234.50`, `+4,210.00`, `0.00`).
    pub text: String,

    /// Whether the amount, as rounded for display, is below zero.
    pub is_negative: bool,
}

fn fraction_digits(amount: &Money, places: Places) -> u32 {
    match places {
        Places::Fixed(places) => places,
        Places::Own => u32::try_from(amount.0.fractional_digit_count().max(0)).unwrap_or(0),
        Places::Natural if amount.0.is_integer() => 0,
        Places::Natural => 2,
    }
}

fn try_format(amount: &Money, style: AmountStyle) -> Result<Amount> {
    let mut decimal = to_decimal(amount, fraction_digits(amount, style.places))?;
    // `to_decimal` has already cleared the sign of a rounded zero.
    let is_negative = decimal.sign() == Sign::Negative;
    let is_positive = !is_negative && !decimal.is_zero();
    decimal.set_sign(Sign::None);
    let magnitude = decimal_formatter(crate::locale())?
        .format(&decimal)
        .to_string();
    let text = if is_negative {
        format!("{MINUS}{magnitude}")
    } else if is_positive && style.plus {
        format!("+{magnitude}")
    } else {
        magnitude
    };
    Ok(Amount { text, is_negative })
}

/// Formats a signed amount for the Locale in effect: grouped and marked by the Locale, at the
/// style's places, with U+2212 for a negative and, if the style asks, `+` for a positive. A value
/// that rounds to zero is neither.
pub fn format_amount(amount: &Money, style: AmountStyle) -> Amount {
    try_format(amount, style).unwrap_or_else(|error| {
        tracing::warn!(%error, "amount formatting failed; using plain text");
        let plain = amount.0.to_plain_string();
        Amount {
            is_negative: plain.starts_with('-'),
            text: plain,
        }
    })
}
