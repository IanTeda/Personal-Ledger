//! Locale-aware upper-casing for headings.

use icu_casemap::CaseMapper;
use icu_locale::LanguageIdentifier;

use super::icu_locale;

/// Upper-cases `text` by the Locale's rules. A Message is written once in sentence case and the
/// renderer upper-cases it where a heading needs it. Falls back to Unicode default casing if the
/// Locale cannot be resolved.
pub fn upper(text: &str) -> String {
    let language: LanguageIdentifier = match icu_locale(crate::locale()) {
        Ok(locale) => locale.id,
        Err(error) => {
            tracing::warn!(%error, "casing Locale unresolved; using default casing");
            return text.to_uppercase();
        }
    };
    CaseMapper::new()
        .uppercase_to_string(text, &language)
        .into_owned()
}
