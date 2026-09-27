//! Colour Themes shared by the Desktop and TUI Clients (ADR-0025).
//!
//! Owns the Colour Role, Colour Variant and Colour Appearance types, the built-in Colour
//! Themes (compiled from `themes/<id>.ini` by `build.rs`), and [`resolve`], which lays
//! `[theme]` overrides over the chosen Colour Variant and calculates every other colour.
//! Pure: no I/O, no logging and no dependency on `lib-config`. Design in
//! `docs/colour-themes-design.md`.

mod colour;
mod parse;
mod resolve;
mod theme;

pub use colour::{Rgba, contrast_ratio, mix};
pub use parse::{ParseError, ThemeFile, parse_hex, parse_theme};
pub use resolve::{
    CHART_SERIES_COUNT, ContrastFailure, InfoToast, NON_TEXT_CONTRAST, ResolveInputs,
    ResolvedColours, Selection, TextShades, ThemeOverrides, resolve,
};
pub use theme::{ColourAppearance, ColourRole, ColourTheme, ColourVariant, Palette};
