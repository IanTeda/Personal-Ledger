//! # Message generator
//!
//! Reads a layer of Fluent Catalogue files (`i18n/<locale>/*.ftl`), checks every Locale against
//! the source Locale (`en-US`), and generates one typed free function per Message and attribute.
//! It is used by `lib-locale`'s `build.rs` and, later, by each bin's `build.rs`.
//!
//! It is a separate crate because a `build.rs` cannot use its own crate's dependencies, and a
//! build-dependency on `lib-locale` would compile Fluent and ICU4X for the host as well.
//!
//! The generator only sees one layer, so it never fails on a term or Message reference it
//! cannot resolve locally (a bin's build cannot see the shared layer). Cross-layer id overlap is
//! caught by the id prefix rule here and by a check at `lib_locale::init`.

mod emit;
mod error;
mod extract;
mod layer;

pub use emit::{Generated, Options, generate, run};
pub use error::{Error, Result};

/// The source Locale: the only complete Catalogue, checked against by every other Locale.
pub const SOURCE_LOCALE: &str = "en-US";
