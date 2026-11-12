//! The Display choices a Presenter draws with (Settings › Display): how a table's rows are spaced
//! and how a status is marked. The choices are `lib_core`'s, which `lib_config` reads from the
//! file too (ADR-0053); they are re-exported here, where the Transactions, Bills and Dashboard
//! Presenters already reach for them, with [`PageDisplay`], the three as one page is handed them.
//! Their labels are the Client's Messages, and density to pixels is the Client's toolkit's; only
//! the Flagged mark every module shares is defined here.

pub use lib_core::{RowDensity, StatusGlyphs};

use lib_core::DateStyle;

/// The Display Preferences a page draws with, as Settings › Display hands them to every page
/// ([`PagePresenter::display`](crate::presenter::PagePresenter::display)) on a change and when a
/// Ledger opens. A page reads only the ones it draws.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PageDisplay {
    /// How dates show and are typed; `None` follows the Locale.
    pub date_style: Option<DateStyle>,
    pub glyphs: StatusGlyphs,
    pub density: RowDensity,
}

/// The Flagged mark, shown apart from the status (Flagged is independent of it): ⚑, or `!`. Every
/// module that marks a flag uses it, so a Transaction's and a Bill's flag read alike.
pub fn flag_glyph(style: StatusGlyphs) -> &'static str {
    match style {
        StatusGlyphs::Unicode => "\u{2691}",
        StatusGlyphs::AsciiFallback => "!",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flag_is_a_flag_in_unicode_and_a_bang_in_ascii() {
        assert_eq!(flag_glyph(StatusGlyphs::Unicode), "\u{2691}");
        assert_eq!(flag_glyph(StatusGlyphs::AsciiFallback), "!");
    }
}
