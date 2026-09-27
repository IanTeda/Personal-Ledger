use std::time::Duration;

/// How serious a Toast is, which fixes its lifetime, glyph and mark colour (ADR-0026).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ToastKind {
    Info,
    Success,
    Warning,
    Error,
}

impl ToastKind {
    pub const ALL: [Self; 4] = [Self::Info, Self::Success, Self::Warning, Self::Error];

    /// How long a Toast of this Kind shows, or `None` for an Error, which stays until dismissed.
    /// Fixed constants rather than settings (#306).
    pub fn lifetime(self) -> Option<Duration> {
        match self {
            Self::Info | Self::Success => Some(Duration::from_secs(4)),
            Self::Warning => Some(Duration::from_secs(8)),
            Self::Error => None,
        }
    }

    pub fn is_sticky(self) -> bool {
        self.lifetime().is_none()
    }

    /// The same glyph in both Clients, so no Kind relies on colour alone. Error is `✗`
    /// (U+2717), kept distinct from the Desktop's `✕` dismiss.
    pub fn glyph(self) -> char {
        match self {
            Self::Info => 'i',
            Self::Success => '✓',
            Self::Warning => '!',
            Self::Error => '✗',
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifetimes_match_the_design() {
        assert_eq!(ToastKind::Info.lifetime(), Some(Duration::from_secs(4)));
        assert_eq!(ToastKind::Success.lifetime(), Some(Duration::from_secs(4)));
        assert_eq!(ToastKind::Warning.lifetime(), Some(Duration::from_secs(8)));
        assert_eq!(ToastKind::Error.lifetime(), None);
    }

    #[test]
    fn only_error_is_sticky() {
        let sticky: Vec<_> = ToastKind::ALL
            .into_iter()
            .filter(|k| k.is_sticky())
            .collect();
        assert_eq!(sticky, vec![ToastKind::Error]);
    }

    #[test]
    fn glyphs_match_the_design() {
        let glyphs: String = ToastKind::ALL.into_iter().map(ToastKind::glyph).collect();
        assert_eq!(glyphs, "i✓!✗");
    }
}
