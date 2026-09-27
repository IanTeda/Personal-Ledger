//! Colour Roles, Colour Variants, Colour Appearances and the built-in Colour Themes.

use crate::colour::Rgba;
use crate::parse::{ROLE_KEYS, VariantRgb};

/// One of the seven stored Colour Roles (ADR-0022). Its [`key`](Self::key) is also its
/// `[theme]` Configuration key and its key in a Colour Theme file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ColourRole {
    Foreground,
    Background,
    Accent,
    Cursor,
    Muted,
    Positive,
    Negative,
}

impl ColourRole {
    /// Every role, in the order [`ROLE_KEYS`] lists their keys.
    pub const ALL: [Self; 7] = [
        Self::Foreground,
        Self::Background,
        Self::Accent,
        Self::Cursor,
        Self::Muted,
        Self::Positive,
        Self::Negative,
    ];

    pub fn key(self) -> &'static str {
        ROLE_KEYS[self as usize]
    }

    /// The role for a `[theme]` key, case-insensitively; `None` for an unknown key.
    pub fn from_key(key: &str) -> Option<Self> {
        let key = key.trim().to_ascii_lowercase();
        ROLE_KEYS
            .iter()
            .position(|k| *k == key)
            .map(|i| Self::ALL[i])
    }
}

/// A Colour Theme's light or dark half.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ColourVariant {
    Light,
    Dark,
}

impl ColourVariant {
    pub fn key(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }
}

/// The `colour_appearance` Preference: which Colour Variant to draw.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ColourAppearance {
    Light,
    Dark,
    /// Follow the OS (or terminal) light/dark setting. A null Preference means this.
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

    /// The Preference value for a stored key; `None` for an unknown one.
    pub fn from_key(key: &str) -> Option<Self> {
        match key.trim().to_ascii_lowercase().as_str() {
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            "system" => Some(Self::System),
            _ => None,
        }
    }

    /// The Colour Variant to draw. `system` is the OS or terminal setting, if one was
    /// detected; with none, System draws Dark.
    pub fn variant(self, system: Option<ColourVariant>) -> ColourVariant {
        match self {
            Self::Light => ColourVariant::Light,
            Self::Dark => ColourVariant::Dark,
            Self::System => system.unwrap_or(ColourVariant::Dark),
        }
    }
}

/// One Colour Variant's seven stored Colour Roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    pub foreground: Rgba,
    pub background: Rgba,
    pub accent: Rgba,
    pub cursor: Rgba,
    pub muted: Rgba,
    pub positive: Rgba,
    pub negative: Rgba,
}

impl Palette {
    /// Builds a palette from roles in [`ROLE_KEYS`] order, as `build.rs` generates them.
    pub(crate) const fn from_rgb(roles: VariantRgb) -> Self {
        const fn c(rgb: [u8; 3]) -> Rgba {
            Rgba::rgb(rgb[0], rgb[1], rgb[2])
        }
        Self {
            foreground: c(roles[0]),
            background: c(roles[1]),
            accent: c(roles[2]),
            cursor: c(roles[3]),
            muted: c(roles[4]),
            positive: c(roles[5]),
            negative: c(roles[6]),
        }
    }

    pub fn get(&self, role: ColourRole) -> Rgba {
        match role {
            ColourRole::Foreground => self.foreground,
            ColourRole::Background => self.background,
            ColourRole::Accent => self.accent,
            ColourRole::Cursor => self.cursor,
            ColourRole::Muted => self.muted,
            ColourRole::Positive => self.positive,
            ColourRole::Negative => self.negative,
        }
    }

    pub fn set(&mut self, role: ColourRole, colour: Rgba) {
        let slot = match role {
            ColourRole::Foreground => &mut self.foreground,
            ColourRole::Background => &mut self.background,
            ColourRole::Accent => &mut self.accent,
            ColourRole::Cursor => &mut self.cursor,
            ColourRole::Muted => &mut self.muted,
            ColourRole::Positive => &mut self.positive,
            ColourRole::Negative => &mut self.negative,
        };
        *slot = colour;
    }
}

/// A built-in Colour Theme. Its display name is not here: it is the Message
/// `colour-theme-<id>` in `lib-locale`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColourTheme {
    /// The file stem of `themes/<id>.ini`, and the `colour_theme` Preference value.
    pub id: &'static str,
    pub light: Palette,
    pub dark: Palette,
}

include!(concat!(env!("OUT_DIR"), "/built_in.rs"));

/// The one built-in Colour Theme held to 7:1 for text rather than 4.5:1.
const HIGH_CONTRAST_ID: &str = "high_contrast";

impl ColourTheme {
    /// Every built-in Colour Theme, the default (Modernist) first.
    pub fn built_in() -> &'static [Self] {
        &BUILT_IN
    }

    /// The default Colour Theme, drawn for a null or unknown `colour_theme` Preference.
    pub fn default_theme() -> &'static Self {
        &BUILT_IN[0]
    }

    /// The built-in Colour Theme with this id. `None` for an id this release doesn't know,
    /// such as one synced from a newer Client; the caller logs that and draws the default.
    pub fn by_id(id: &str) -> Option<&'static Self> {
        BUILT_IN.iter().find(|theme| theme.id == id)
    }

    pub fn palette(&self, variant: ColourVariant) -> Palette {
        match variant {
            ColourVariant::Light => self.light,
            ColourVariant::Dark => self.dark,
        }
    }

    /// The contrast ratio text must reach in this Colour Theme.
    pub fn text_contrast(&self) -> f32 {
        if self.id == HIGH_CONTRAST_ID {
            7.0
        } else {
            4.5
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_built_in_themes_modernist_first() {
        let ids: Vec<_> = ColourTheme::built_in().iter().map(|t| t.id).collect();
        assert_eq!(
            ids,
            [
                "modernist",
                "catppuccin",
                "gruvbox",
                "high_contrast",
                "nord"
            ]
        );
        assert_eq!(ColourTheme::default_theme().id, "modernist");
    }

    #[test]
    fn built_in_values_come_from_the_ini_files() {
        let modernist = ColourTheme::by_id("modernist").unwrap();
        assert_eq!(modernist.light.foreground, Rgba::rgb(0x20, 0x1e, 0x1d));
        assert_eq!(modernist.dark.negative, Rgba::rgb(0xff, 0x7a, 0x66));
        assert!(ColourTheme::by_id("solarized").is_none());
    }

    #[test]
    fn role_keys_round_trip() {
        for role in ColourRole::ALL {
            assert_eq!(ColourRole::from_key(role.key()), Some(role));
        }
        assert_eq!(ColourRole::from_key(" Accent "), Some(ColourRole::Accent));
        assert_eq!(ColourRole::from_key("warning"), None);
    }

    #[test]
    fn system_with_no_detected_setting_is_dark() {
        assert_eq!(ColourAppearance::System.variant(None), ColourVariant::Dark);
        assert_eq!(
            ColourAppearance::System.variant(Some(ColourVariant::Light)),
            ColourVariant::Light
        );
        assert_eq!(
            ColourAppearance::Light.variant(Some(ColourVariant::Dark)),
            ColourVariant::Light
        );
        assert_eq!(ColourAppearance::default(), ColourAppearance::System);
        assert_eq!(
            ColourAppearance::from_key("SYSTEM"),
            Some(ColourAppearance::System)
        );
    }
}
