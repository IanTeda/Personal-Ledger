//! Embedded Catalogues, one bundle per Locale, and the process-wide loader.
//!
//! Layers are the files a crate embeds (the generated `LAYER` constants). The loader composes
//! every layer for a Locale into one bundle, adding the fallback chain lowest priority first
//! with `add_resource_overriding`, so a sparse `en-AU` overrides `en-GB`, which overrides
//! `en-US`.

mod bundle;
mod loader;
mod pseudo;

pub use loader::{format, init, init_with_layers, locale, with_locale};

/// The embedded files of one crate's Catalogue: `(locale tag, [(file name, contents)])`.
pub type Layer = &'static [(&'static str, &'static [(&'static str, &'static str)])];

/// A Message argument. Only counts go in as numbers, for plural selectors; money, dates and
/// other formatted values arrive as text from the formatting layer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Arg<'a> {
    /// A plural count.
    Int(i64),

    /// Any other value.
    Text(&'a str),
}
