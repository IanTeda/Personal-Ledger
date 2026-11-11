//! # Display choices
//!
//! Row density and Status glyphs, a device's settings kept in `[Display]` as `row_density`
//! and `status_glyphs` (ADR-0053). Here, beside [`crate::DateStyle`], because `lib-config` reads
//! them from the file and the Presenters draw with them (through `lib_viper::display`), and the
//! two must agree on one vocabulary: the TUI and the Desktop share the `status_glyphs` key, so a
//! value one writes is the other's on its next start.

/// How tightly a table's rows are spaced: Settings › Display's Row density.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RowDensity {
    Compact,
    #[default]
    Regular,
    Roomy,
}

impl RowDensity {
    pub const ALL: [RowDensity; 3] = [Self::Compact, Self::Regular, Self::Roomy];

    /// The value as the file spells it.
    pub fn key(self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Regular => "regular",
            Self::Roomy => "roomy",
        }
    }

    /// The value `key` spells, ignoring case and surrounding space.
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|density| density.key().eq_ignore_ascii_case(key.trim()))
    }
}

/// How a status is marked: Unicode glyphs, or plain ASCII stand-ins for a font that lacks them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusGlyphs {
    #[default]
    Unicode,
    /// Spelt `ascii` in the file, as the TUI has always written it.
    AsciiFallback,
}

impl StatusGlyphs {
    pub const ALL: [StatusGlyphs; 2] = [Self::Unicode, Self::AsciiFallback];

    /// The value as the file and the TUI's `:set glyphs` spell it.
    pub fn key(self) -> &'static str {
        match self {
            Self::Unicode => "unicode",
            Self::AsciiFallback => "ascii",
        }
    }

    /// The value `key` spells, ignoring case and surrounding space.
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|glyphs| glyphs.key().eq_ignore_ascii_case(key.trim()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regular_and_unicode_are_the_defaults() {
        assert_eq!(RowDensity::default(), RowDensity::Regular);
        assert_eq!(StatusGlyphs::default(), StatusGlyphs::Unicode);
    }

    #[test]
    fn status_glyphs_keep_the_spellings_the_tui_writes() {
        assert_eq!(StatusGlyphs::Unicode.key(), "unicode");
        assert_eq!(StatusGlyphs::AsciiFallback.key(), "ascii");
        assert_eq!(
            StatusGlyphs::from_key("ASCII"),
            Some(StatusGlyphs::AsciiFallback)
        );
        assert_eq!(
            StatusGlyphs::from_key(" unicode "),
            Some(StatusGlyphs::Unicode)
        );
        assert_eq!(StatusGlyphs::from_key("ascii fallback"), None);
    }

    #[test]
    fn row_density_reads_back_each_spelling() {
        for density in RowDensity::ALL {
            assert_eq!(RowDensity::from_key(density.key()), Some(density));
        }
        assert_eq!(RowDensity::from_key("Roomy"), Some(RowDensity::Roomy));
        assert_eq!(RowDensity::from_key("cosy"), None);
    }
}
