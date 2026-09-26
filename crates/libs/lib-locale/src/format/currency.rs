//! Currency and quantity formatting: the Unit decides what is shown, the Locale how.
//!
//! ICU4X's currency formatter lives in `icu_experimental`, which is not semver-stable, so every
//! use of it is confined to this file and pinned in `Cargo.toml`.

use std::rc::Rc;

use icu_experimental::dimension::currency::formatter::CurrencyFormatter;
use icu_locale::preferences::extensions::unicode::keywords::CurrencyType;
use lib_core::{Money, UnitKind};

use super::number::to_decimal;
use super::{Cache, cached, icu_locale};
use crate::error::{Error, Result};
use crate::locale::Locale;

thread_local! {
    static FORMATTERS: Cache<(Locale, String), CurrencyFormatter<icu_decimal::DecimalFormatter>> =
        Cache::default();
}

/// What a Unit contributes to formatting: its code, kind and how many decimal places it is
/// naturally shown with. Built by a bin from its stored Unit, so this crate needs no database
/// types.
#[derive(Debug, Clone, Copy)]
pub struct Unit<'a> {
    /// The Unit code (`AUD`, `BTC`, `AAPL`).
    pub code: &'a str,

    /// The Unit's kind; only `Fiat` uses the currency formatter.
    pub kind: &'a UnitKind,

    /// Decimal places the Unit is shown with (2 for AUD, 8 for BTC, 0 for JPY).
    pub decimal_places: u32,
}

impl<'a> Unit<'a> {
    /// Builds a Unit from its parts.
    pub fn new(code: &'a str, kind: &'a UnitKind, decimal_places: u32) -> Self {
        Unit {
            code,
            kind,
            decimal_places,
        }
    }
}

fn currency_type(code: &str) -> Result<CurrencyType> {
    CurrencyType::try_from_str(code)
        .map_err(|error| Error::Format(format!("`{code}` is not a currency code: {error}")))
}

fn formatter(
    locale: Locale,
    code: &str,
) -> Result<Rc<CurrencyFormatter<icu_decimal::DecimalFormatter>>> {
    cached(&FORMATTERS, (locale, code.to_string()), || {
        let prefs = (&icu_locale(locale)?).into();
        CurrencyFormatter::try_new_symbol(prefs, currency_type(code)?, Default::default()).map_err(
            |error| {
                Error::Format(format!(
                    "currency formatter for {code} in {locale}: {error}"
                ))
            },
        )
    })
}

fn try_format_fiat(amount: &Money, unit: Unit<'_>) -> Result<String> {
    let decimal = to_decimal(amount, unit.decimal_places)?;
    let formatter = formatter(crate::locale(), unit.code)?;
    Ok(formatter.format_fixed_decimal(&decimal).to_string())
}

fn format_quantity(amount: &Money, unit: Unit<'_>) -> String {
    let quantity = super::number::format_number(amount, unit.decimal_places);
    crate::msg::unit_quantity(&quantity, unit.code)
}

/// Formats an amount in its Unit. A fiat Unit goes through the currency formatter (`A$1,234.50`
/// for an AUD amount viewed in `en-US`, `$1,234.50` in `en-AU`). Any other Unit (a ticker,
/// bitcoin) formats the quantity as a decimal and places the Unit code with a Message.
///
/// For fiat, ICU4X applies the currency's own CLDR fraction digits after the Unit's
/// `decimal_places` rounding, so those digits win when they differ. A non-fiat Unit's
/// `decimal_places` is used as configured.
pub fn format_money(amount: &Money, unit: &Unit<'_>) -> String {
    if *unit.kind == UnitKind::Fiat {
        match try_format_fiat(amount, *unit) {
            Ok(text) => return text,
            Err(error) => {
                tracing::warn!(%error, code = unit.code, "currency formatting failed; using a quantity")
            }
        }
    }
    format_quantity(amount, *unit)
}
