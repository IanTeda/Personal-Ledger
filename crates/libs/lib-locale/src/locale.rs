//! The supported Locales, negotiation and the fallback chain.

use std::fmt;
use std::str::FromStr;

use unic_langid::LanguageIdentifier;

use crate::error::{Error, Result};

/// A Locale this crate ships a Catalogue for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Locale {
    /// United States English, the source Locale and the only complete Catalogue.
    EnUs,

    /// British English, a sparse override of `en-US`.
    EnGb,

    /// Australian English, a sparse override of `en-GB`.
    EnAu,

    /// The pseudo-Locale, generated from `en-US` at load time to expose untranslated text and
    /// layout overflow. Reachable only by exact request.
    EnXa,
}

impl Locale {
    /// Every supported Locale.
    pub const SUPPORTED: [Locale; 4] = [Locale::EnUs, Locale::EnGb, Locale::EnAu, Locale::EnXa];

    /// The Locale used when a request is unsupported or unreadable, and the source Locale.
    pub const DEFAULT: Locale = Locale::EnUs;

    /// The canonical BCP-47 tag.
    pub fn tag(self) -> &'static str {
        match self {
            Locale::EnUs => "en-US",
            Locale::EnGb => "en-GB",
            Locale::EnAu => "en-AU",
            Locale::EnXa => "en-XA",
        }
    }

    /// Picks a supported Locale for a requested tag: an exact match wins, anything else
    /// (`en-NZ`, `fr-FR`, `C`, `POSIX`) becomes `en-US`. `en-XA` is only chosen by exact request.
    pub fn negotiate(requested: &str) -> Locale {
        let Ok(parsed) = LanguageIdentifier::from_str(requested.trim()) else {
            return Locale::DEFAULT;
        };
        let canonical = parsed.to_string();
        Locale::SUPPORTED
            .into_iter()
            .find(|locale| locale.tag() == canonical)
            .unwrap_or(Locale::DEFAULT)
    }

    /// The Locales consulted for a Message, highest priority first, ending in `en-US`:
    /// `en-AU`, `en-GB`, `en-US` for `en-AU`, and so on.
    pub fn fallback_chain(self) -> &'static [Locale] {
        match self {
            Locale::EnUs => &[Locale::EnUs],
            Locale::EnGb => &[Locale::EnGb, Locale::EnUs],
            Locale::EnAu => &[Locale::EnAu, Locale::EnGb, Locale::EnUs],
            Locale::EnXa => &[Locale::EnXa, Locale::EnUs],
        }
    }

    /// Whether this is the generated pseudo-Locale.
    pub fn is_pseudo(self) -> bool {
        self == Locale::EnXa
    }

    /// The Locale whose plural rules and formatting data apply. ICU4X has no `en-XA` data, so
    /// the pseudo-Locale uses `en-US`.
    pub fn formatting_locale(self) -> Locale {
        if self.is_pseudo() { Locale::EnUs } else { self }
    }

    pub(crate) fn language_identifier(self) -> Result<LanguageIdentifier> {
        LanguageIdentifier::from_str(self.formatting_locale().tag()).map_err(|error| {
            Error::Generic(format!("invalid built-in tag {}: {error}", self.tag()))
        })
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.tag())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_matches_win() {
        for locale in Locale::SUPPORTED {
            assert_eq!(Locale::negotiate(locale.tag()), locale);
        }
    }

    #[test]
    fn case_is_normalised() {
        assert_eq!(Locale::negotiate("en-au"), Locale::EnAu);
        assert_eq!(Locale::negotiate("EN-gb"), Locale::EnGb);
    }

    #[test]
    fn everything_else_becomes_en_us() {
        for requested in ["en-NZ", "fr-FR", "en", "C", "POSIX", "", "not a tag!!"] {
            assert_eq!(Locale::negotiate(requested), Locale::EnUs, "{requested}");
        }
    }

    #[test]
    fn chains_end_in_the_source_locale() {
        assert_eq!(
            Locale::EnAu.fallback_chain(),
            &[Locale::EnAu, Locale::EnGb, Locale::EnUs]
        );
        assert_eq!(Locale::EnGb.fallback_chain(), &[Locale::EnGb, Locale::EnUs]);
        assert_eq!(Locale::EnXa.fallback_chain(), &[Locale::EnXa, Locale::EnUs]);
        assert_eq!(Locale::EnUs.fallback_chain(), &[Locale::EnUs]);
    }

    #[test]
    fn pseudo_locale_formats_as_en_us() {
        assert_eq!(Locale::EnXa.formatting_locale(), Locale::EnUs);
        assert_eq!(Locale::EnAu.formatting_locale(), Locale::EnAu);
    }
}
