//! # Colour Theme Role Overrides
//!
//! Configuration for the `[theme]`, `[theme.light]` and `[theme.dark]` sections (ADR-0023):
//! per-Client overrides of the seven stored Colour Roles, laid over whichever Colour Theme
//! the synced Preferences pick. A bare `[theme]` key applies to both Colour Variants; a
//! `[theme.<variant>]` key to one, and wins over the bare key there.
//!
//! ## Configuration File Example
//!
//! ```ini
//! [theme]
//! accent = "#1F6FEB"
//!
//! [theme.dark]
//! accent = "#58A6FF"
//! ```
//!
//! ## Environment Variables
//!
//! ```bash
//! PERSONAL_LEDGER_THEME__ACCENT=1F6FEB
//! PERSONAL_LEDGER_THEME__DARK__ACCENT=58A6FF
//! ```
//!
//! Both spellings meet in the same place: `config` reads the INI section `[theme.dark]` as
//! the dotted path `theme.dark`, the same nesting `THEME__DARK__` produces.
//!
//! A bad value or unknown key is dropped rather than failing the parse, because a typo in a
//! colour should never stop the Client starting. The rejects are kept on [`ThemeConfig`]
//! and logged by [`ThemeConfig::warn_invalid`], since parsing happens before tracing is
//! initialised (the log level is itself Configuration).

use config::{Map, Value};
use lib_colour_theme::{ColourRole, ThemeOverrides};
use lib_core::HexColor;

/// The configuration section holding the role overrides.
const SECTION: &str = "theme";

/// A `[theme]` entry that was dropped, kept so it can be logged once tracing is up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidThemeEntry {
    /// The dotted key as written, e.g. `theme.dark.accent`.
    pub key: String,
    /// The value as written, or a description when it wasn't a plain value.
    pub value: String,
}

/// The parsed `[theme]` sections: the overrides `lib_colour_theme::resolve` takes, plus
/// every entry that was dropped.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ThemeConfig {
    pub overrides: ThemeOverrides,
    pub invalid: Vec<InvalidThemeEntry>,
}

impl ThemeConfig {
    /// Read the `[theme]` sections out of the merged layered configuration. Never fails.
    pub(crate) fn from_config(config: &config::Config) -> Self {
        let mut theme = Self::default();
        let Ok(table) = config.get_table(SECTION) else {
            return theme;
        };

        for (key, value) in table {
            match key.to_ascii_lowercase().as_str() {
                variant @ ("light" | "dark") => match value.clone().into_table() {
                    Ok(variant_table) => {
                        let prefix = format!("{SECTION}.{variant}");
                        let target = if variant == "light" {
                            &mut theme.overrides.light
                        } else {
                            &mut theme.overrides.dark
                        };
                        Self::insert_roles(&prefix, variant_table, target, &mut theme.invalid);
                    }
                    Err(_) => theme.invalid.push(InvalidThemeEntry {
                        key: format!("{SECTION}.{key}"),
                        value: value.to_string(),
                    }),
                },
                _ => Self::insert_role(
                    SECTION,
                    &key,
                    value,
                    &mut theme.overrides.both,
                    &mut theme.invalid,
                ),
            }
        }

        theme
    }

    fn insert_roles(
        prefix: &str,
        table: Map<String, Value>,
        target: &mut std::collections::BTreeMap<ColourRole, HexColor>,
        invalid: &mut Vec<InvalidThemeEntry>,
    ) {
        for (key, value) in table {
            Self::insert_role(prefix, &key, value, target, invalid);
        }
    }

    fn insert_role(
        prefix: &str,
        key: &str,
        value: Value,
        target: &mut std::collections::BTreeMap<ColourRole, HexColor>,
        invalid: &mut Vec<InvalidThemeEntry>,
    ) {
        let raw = value.to_string();
        let parsed = ColourRole::from_key(key).zip(
            value
                .into_string()
                .ok()
                .and_then(|s| HexColor::parse(s).ok()),
        );
        match parsed {
            Some((role, colour)) => {
                target.insert(role, colour);
            }
            None => invalid.push(InvalidThemeEntry {
                key: format!("{prefix}.{key}"),
                value: raw,
            }),
        }
    }

    /// Log each dropped entry at `warn`. Call once, after tracing is initialised.
    pub fn warn_invalid(&self) {
        for entry in &self.invalid {
            tracing::warn!(
                key = %entry.key,
                value = %entry.value,
                "Ignoring [theme] entry: not a Colour Role key with a hex colour value"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ini(ini: &str) -> ThemeConfig {
        let config = config::Config::builder()
            .add_source(config::File::from_str(ini, config::FileFormat::Ini))
            .build()
            .unwrap();
        ThemeConfig::from_config(&config)
    }

    fn hex(value: &str) -> HexColor {
        HexColor::parse(value).unwrap()
    }

    #[test]
    fn no_theme_section_is_empty() {
        let theme = parse_ini("[other]\nkey = \"value\"\n");
        assert!(theme.overrides.is_empty());
        assert!(theme.invalid.is_empty());
    }

    #[test]
    fn bare_keys_apply_to_both_variants() {
        let theme = parse_ini("[theme]\naccent = \"#1f6feb\"\nMuted = \"777777\"\n");
        assert_eq!(
            theme.overrides.both.get(&ColourRole::Accent),
            Some(&hex("#1F6FEB"))
        );
        assert_eq!(
            theme.overrides.both.get(&ColourRole::Muted),
            Some(&hex("#777777"))
        );
        assert!(theme.invalid.is_empty());
    }

    #[test]
    fn unknown_keys_and_bad_hex_are_dropped_and_recorded() {
        let theme = parse_ini(
            "[theme]\naccent = \"#1F6FEB\"\nborder = \"#000000\"\npositive = \"green\"\n",
        );
        assert_eq!(theme.overrides.both.len(), 1);
        let keys: Vec<_> = theme.invalid.iter().map(|e| e.key.as_str()).collect();
        assert!(keys.contains(&"theme.border"));
        assert!(keys.contains(&"theme.positive"));
        assert_eq!(theme.invalid.len(), 2);
    }

    #[test]
    fn variant_sections_override_bare_keys_for_their_variant() {
        let theme = parse_ini(
            "[theme]\naccent = \"#111111\"\n[theme.dark]\naccent = \"#222222\"\n[theme.light]\nnope = \"#333333\"\n",
        );
        let dark = lib_colour_theme::ColourVariant::Dark;
        let light = lib_colour_theme::ColourVariant::Light;
        assert_eq!(
            theme.overrides.get(dark, ColourRole::Accent),
            Some(&hex("#222222"))
        );
        assert_eq!(
            theme.overrides.get(light, ColourRole::Accent),
            Some(&hex("#111111"))
        );
        assert_eq!(theme.invalid.len(), 1);
        assert_eq!(theme.invalid[0].key, "theme.light.nope");
    }
}
