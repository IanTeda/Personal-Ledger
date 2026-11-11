//! # The `[Display]` section
//!
//! The keys Settings › Display shows (ADR-0057): the Colour Theme and Appearance, the date style,
//! Row density, Status glyphs, and the TUI's own hint bar and terminal colours. Each is spelled in
//! the file as `:set` takes it.

use lib_colour_theme::ColourAppearance;
use lib_core::{DateStyle, RowDensity, StatusGlyphs};

use crate::general::by_key;

/// Configuration for the `[Display]` section.
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(default)]
pub struct DisplayConfig {
    /// The chosen Colour Theme's id. Unset is the default Colour Theme; kept raw, so an id from a
    /// newer release survives a run of this one (ADR-0023).
    pub colour_theme: Option<String>,

    /// Which Colour Variant to draw: `light`, `dark` or `system`, the default.
    #[serde(with = "appearance")]
    pub colour_appearance: ColourAppearance,

    /// How dates show: `short`, `medium`, `long` or `iso`, or `default`, the Locale's (ADR-0021),
    /// which unset means too.
    #[serde(with = "date_style")]
    pub date_style: Option<DateStyle>,

    /// How tightly a table's rows are spaced: `compact`, `regular`, the default, or `roomy`. Read
    /// only by the Desktop, which has the tables it spaces.
    #[serde(with = "row_density")]
    pub row_density: RowDensity,

    /// Draw status glyphs as Unicode or plain ASCII: `unicode`, the default, or `ascii`. Both
    /// Clients read and write it.
    #[serde(with = "status_glyphs")]
    pub status_glyphs: StatusGlyphs,

    /// Whether the TUI's footer shows the key hints. Read only by the TUI.
    pub hint_bar: bool,

    /// Draw with the terminal's own ANSI colours instead of the Colour Theme (ADR-0024).
    /// Read only by the TUI.
    pub terminal_colours: bool,
}

/// The value `date_style` and `:set date-style` spell the Locale's default with.
pub const DATE_STYLE_DEFAULT: &str = "default";

/// The value the file and `:set date-style` spell `style` with.
pub fn date_style_key(style: Option<DateStyle>) -> &'static str {
    style.map_or(DATE_STYLE_DEFAULT, |style| style.as_str())
}

/// `colour_appearance` by [`ColourAppearance::key`].
mod appearance {
    use super::ColourAppearance;

    pub fn serialize<S: serde::Serializer>(
        value: &ColourAppearance,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(value.key())
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<ColourAppearance, D::Error> {
        super::by_key(deserializer, ColourAppearance::from_key)
    }
}

/// `date_style` by [`super::date_style_key`].
mod date_style {
    use super::DateStyle;

    pub fn serialize<S: serde::Serializer>(
        value: &Option<DateStyle>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(super::date_style_key(*value))
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<DateStyle>, D::Error> {
        super::by_key(deserializer, |key| {
            let key = key.trim();
            if key.eq_ignore_ascii_case(super::DATE_STYLE_DEFAULT) {
                Some(None)
            } else {
                key.parse().ok().map(Some)
            }
        })
    }
}

/// `status_glyphs` by [`StatusGlyphs::key`].
mod status_glyphs {
    use super::StatusGlyphs;

    pub fn serialize<S: serde::Serializer>(
        value: &StatusGlyphs,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(value.key())
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<StatusGlyphs, D::Error> {
        super::by_key(deserializer, StatusGlyphs::from_key)
    }
}

/// `row_density` by [`RowDensity::key`].
mod row_density {
    use super::RowDensity;

    pub fn serialize<S: serde::Serializer>(
        value: &RowDensity,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(value.key())
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<RowDensity, D::Error> {
        super::by_key(deserializer, RowDensity::from_key)
    }
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            colour_theme: None,
            colour_appearance: ColourAppearance::System,
            date_style: None,
            row_density: RowDensity::Regular,
            status_glyphs: StatusGlyphs::Unicode,
            hint_bar: true,
            terminal_colours: false,
        }
    }
}

impl DisplayConfig {
    /// The chosen Colour Theme's id, or `None` for the default Colour Theme.
    pub fn colour_theme(&self) -> Option<&str> {
        self.colour_theme.as_deref()
    }

    /// Which Colour Variant to draw.
    pub fn colour_appearance(&self) -> ColourAppearance {
        self.colour_appearance
    }

    /// The date style, or `None` for the Locale's default.
    pub fn date_style(&self) -> Option<DateStyle> {
        self.date_style
    }

    /// How tightly the Desktop spaces a table's rows.
    pub fn row_density(&self) -> RowDensity {
        self.row_density
    }

    /// How status glyphs are drawn.
    pub fn status_glyphs(&self) -> StatusGlyphs {
        self.status_glyphs
    }

    /// Whether the TUI's footer shows the key hints.
    pub fn hint_bar(&self) -> bool {
        self.hint_bar
    }

    /// Whether the TUI draws with the terminal's own colours rather than the Colour Theme.
    pub fn terminal_colours(&self) -> bool {
        self.terminal_colours
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    /// The `[Display]` section `text` and `env` give.
    fn parsed(text: &str, env: &[(&str, &str)]) -> DisplayConfig {
        crate::ledger::parsed_for_test(text, env)
            .display_config()
            .clone()
    }

    #[test]
    fn the_display_settings_default_with_no_section_at_all() {
        let section = parsed("", &[]);
        assert_eq!(section, DisplayConfig::default());
        assert_eq!(section.colour_theme(), None);
        assert_eq!(section.colour_appearance(), ColourAppearance::System);
        assert_eq!(section.date_style(), None);
        assert_eq!(section.row_density(), RowDensity::Regular);
        assert_eq!(section.status_glyphs(), StatusGlyphs::Unicode);
        assert!(section.hint_bar());
        assert!(!section.terminal_colours());
    }

    #[test]
    fn the_display_settings_read_from_the_file_as_set_spells_them() {
        let section = parsed(
            "[Display]\ncolour_theme = nord\ncolour_appearance = Light\ndate_style = ISO\n\
             hint_bar = false\nterminal_colours = true\n",
            &[],
        );
        assert_eq!(section.colour_theme(), Some("nord"));
        assert_eq!(section.colour_appearance(), ColourAppearance::Light);
        assert_eq!(section.date_style(), Some(DateStyle::Iso));
        assert!(!section.hint_bar());
        assert!(section.terminal_colours());

        let section = parsed("[Display]\ndate_style = default\n", &[]);
        assert_eq!(section.date_style(), None);
    }

    #[test]
    fn row_density_and_status_glyphs_read_as_the_file_spells_them_in_any_case() {
        for (text, density, glyphs) in [
            ("compact", RowDensity::Compact, "ascii"),
            ("Roomy", RowDensity::Roomy, "ASCII"),
            ("regular", RowDensity::Regular, "Ascii"),
        ] {
            let section = parsed(
                &format!("[Display]\nrow_density = {text}\nstatus_glyphs = {glyphs}\n"),
                &[],
            );
            assert_eq!(section.row_density(), density, "{text}");
            assert_eq!(
                section.status_glyphs(),
                StatusGlyphs::AsciiFallback,
                "{glyphs}"
            );
        }
        let section = parsed("[Display]\nstatus_glyphs = unicode\n", &[]);
        assert_eq!(section.status_glyphs(), StatusGlyphs::Unicode);
    }

    #[test]
    fn an_environment_variable_outranks_the_file_for_a_display_setting() {
        let section = parsed(
            "[Display]\nrow_density = compact\nstatus_glyphs = ascii\ndate_style = long\n",
            &[
                ("PERSONAL_LEDGER_DISPLAY__ROW_DENSITY", "roomy"),
                ("PERSONAL_LEDGER_DISPLAY__STATUS_GLYPHS", "unicode"),
                ("PERSONAL_LEDGER_DISPLAY__COLOUR_APPEARANCE", "dark"),
                ("PERSONAL_LEDGER_DISPLAY__HINT_BAR", "false"),
                ("PERSONAL_LEDGER_DISPLAY__TERMINAL_COLOURS", "true"),
            ],
        );
        assert_eq!(section.row_density(), RowDensity::Roomy);
        assert_eq!(section.status_glyphs(), StatusGlyphs::Unicode);
        assert_eq!(section.colour_appearance(), ColourAppearance::Dark);
        assert_eq!(section.date_style(), Some(DateStyle::Long));
        assert!(!section.hint_bar());
        assert!(section.terminal_colours());
    }

    #[test]
    fn a_display_setting_value_it_doesnt_take_is_a_configuration_error() {
        for text in [
            "[Display]\ndate_style = fancy\n",
            "[Display]\ncolour_appearance = sepia\n",
            "[Display]\nrow_density = cosy\n",
            "[Display]\nstatus_glyphs = ascii fallback\n",
        ] {
            let dir = tempfile::TempDir::new().unwrap();
            let path = dir.path().join("personal-ledger.conf");
            std::fs::write(&path, text).unwrap();
            assert!(
                crate::Config::parse_with_detector(Some(&path), Some(HashMap::new()), || None)
                    .is_err(),
                "{text}"
            );
        }
    }

    #[test]
    fn config_serialization_roundtrip() {
        let config = DisplayConfig::default();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: DisplayConfig = serde_json::from_str(&serialized).unwrap();
        assert_eq!(config, deserialized);
    }
}
