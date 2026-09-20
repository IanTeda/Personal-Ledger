//! # Locale, Catalogues and formatting
//!
//! `lib-locale` is the shared localisation crate for the desktop and the TUI. Despite the name,
//! it owns three things:
//!
//! - **Locale resolution**: the supported Locales (`en-US`, `en-GB`, `en-AU` and the `en-XA`
//!   pseudo-Locale), negotiation from a requested tag, and the explicit fallback chain
//!   (`en-AU` to `en-GB` to `en-US`).
//! - **Catalogues**: Fluent Messages embedded at compile time, one bundle per Locale composed
//!   from a shared layer plus an optional per-bin layer, looked up through generated typed
//!   accessors in [`msg`].
//! - **Formatting**: number, date, currency and casing, driven by the Locale in effect and backed
//!   by ICU4X, in [`format`].
//!
//! A bin calls [`init`] once at startup, then any [`msg`] accessor:
//!
//! ```
//! let locale = lib_locale::init("en-AU");
//! assert_eq!(locale, lib_locale::Locale::EnAu);
//! assert_eq!(lib_locale::msg::nav_accounts(), "Accounts");
//! ```
//!
//! No Fluent, ICU or `unic-langid` type appears in the public API, so those dependencies can
//! change without touching the bins.
//!
//! `gpui_component` has its own `locale()` function. Import `lib_locale::Locale` and call
//! `lib_locale::init` explicitly rather than glob-importing both crates.

mod catalogue;
mod error;
pub mod format;
mod locale;
pub mod msg;
mod rich;
mod vocabulary;

pub use catalogue::{init, init_with_layers, locale, with_locale};
pub use error::{Error, Result};
pub use locale::Locale;
pub use rich::Segment;
pub use vocabulary::{Label, flagged_label};

/// Support items for generated accessors, including those a bin's own `build.rs` generates.
/// Not part of the API to call directly.
#[doc(hidden)]
pub mod runtime {
    pub use crate::catalogue::{Arg, Layer, format, format_rich};
    pub use crate::rich::Segment;
}
