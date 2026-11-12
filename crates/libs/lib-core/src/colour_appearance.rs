//! The `colour_appearance` Configuration: which Colour Variant to draw.

/// The `colour_appearance` Configuration (not a Preference, `GLOSSARY.md`): which Colour Variant
/// to draw. Here rather than in `lib-colour-theme` so the Configuration key catalogue
/// (`crate::configuration`) can hold it (ADR-0058); `lib-colour-theme` re-exports it and turns
/// it into a Colour Variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ColourAppearance {
    Light,
    Dark,
    /// Follow the OS (or terminal) light/dark setting. An unset key means this.
    #[default]
    System,
}

impl ColourAppearance {
    pub fn key(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::System => "system",
        }
    }

    /// The value for a stored key; `None` for an unknown one.
    pub fn from_key(key: &str) -> Option<Self> {
        match key.trim().to_ascii_lowercase().as_str() {
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            "system" => Some(Self::System),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_is_the_default_and_keys_read_back_ignoring_case() {
        assert_eq!(ColourAppearance::default(), ColourAppearance::System);
        for appearance in [
            ColourAppearance::Light,
            ColourAppearance::Dark,
            ColourAppearance::System,
        ] {
            assert_eq!(
                ColourAppearance::from_key(appearance.key()),
                Some(appearance)
            );
        }
        assert_eq!(
            ColourAppearance::from_key(" SYSTEM "),
            Some(ColourAppearance::System)
        );
        assert_eq!(ColourAppearance::from_key("sepia"), None);
    }
}
