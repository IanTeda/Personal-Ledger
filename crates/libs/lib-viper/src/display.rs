//! The Display Preferences a Presenter draws with (Settings › Display): how a table's rows are
//! spaced and how a status is marked. Here rather than in a feature crate, since the Transactions
//! and Bills Presenters both take them and no feature crate depends on another. Their labels are
//! the Client's Messages, and density to pixels is the Client's toolkit's; only the choices and the
//! Flagged mark every module shares are here.

/// How tightly a table's rows are spaced: the Display page's "Row density" control (`regular` the
/// mockup's own `checked` option).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RowDensity {
    Compact,
    #[default]
    Regular,
    Roomy,
}

impl RowDensity {
    pub const ALL: [RowDensity; 3] = [Self::Compact, Self::Regular, Self::Roomy];
}

/// How a status is marked: the Display page's "Status glyphs" radio group (`unicode` the mockup's
/// own `checked` option).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusGlyphs {
    #[default]
    Unicode,
    AsciiFallback,
}

impl StatusGlyphs {
    pub const ALL: [StatusGlyphs; 2] = [Self::Unicode, Self::AsciiFallback];
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
    fn the_defaults_are_the_mockups_checked_options() {
        assert_eq!(RowDensity::default(), RowDensity::Regular);
        assert_eq!(StatusGlyphs::default(), StatusGlyphs::Unicode);
    }

    #[test]
    fn the_flag_is_a_flag_in_unicode_and_a_bang_in_ascii() {
        assert_eq!(flag_glyph(StatusGlyphs::Unicode), "\u{2691}");
        assert_eq!(flag_glyph(StatusGlyphs::AsciiFallback), "!");
    }
}
