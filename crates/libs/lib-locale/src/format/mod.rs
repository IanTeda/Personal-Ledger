//! Locale-aware formatting of numbers, dates, currency amounts and casing.
//!
//! Every function formats for the Locale in effect ([`crate::locale`]), so a call site never
//! passes one. `en-XA` formats as `en-US`, because ICU4X has no `en-XA` data. Amounts stay
//! exact: [`Money`](lib_core::Money) is converted through its plain decimal string, never
//! `Display` (which can emit `1e+30`) and never `f64`.
//!
//! A formatting failure never panics or reaches the caller: it is logged and the value falls
//! back to plain, unlocalised text.

mod casing;
mod currency;
mod date;
mod input;
mod number;

pub use casing::upper;
pub use currency::{Unit, format_money};
pub use date::{format_date, format_month, format_year_month};
pub use input::{
    DateField, DateInputError, DateInputOptions, date_error_message, format_date_input, parse_date,
    parse_date_with,
};
pub use number::format_number;

use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;
use std::thread::LocalKey;

use crate::error::{Error, Result};
use icu_locale::Locale as IcuLocale;

use crate::locale::Locale;

/// The ICU4X locale to format with: the pseudo-Locale uses `en-US`.
pub(crate) fn icu_locale(locale: Locale) -> Result<IcuLocale> {
    IcuLocale::try_from_str(locale.formatting_locale().tag())
        .map_err(|error| Error::Format(format!("invalid locale {}: {error}", locale.tag())))
}

/// A per-thread formatter cache, keyed by whatever decides the formatter (Locale, date style,
/// currency). ICU4X formatters are not `Send` or `Sync`, so each thread keeps its own.
pub(crate) type Cache<K, V> = RefCell<HashMap<K, Rc<V>>>;

/// Returns the cached formatter for `key` on this thread, building it on first use.
pub(crate) fn cached<K: Eq + Hash + Clone, V: 'static>(
    cache: &'static LocalKey<Cache<K, V>>,
    key: K,
    build: impl FnOnce() -> Result<V>,
) -> Result<Rc<V>> {
    cache.with(|cell| {
        if let Some(found) = cell.borrow().get(&key) {
            return Ok(Rc::clone(found));
        }
        let built = Rc::new(build()?);
        cell.borrow_mut().insert(key, Rc::clone(&built));
        Ok(built)
    })
}
