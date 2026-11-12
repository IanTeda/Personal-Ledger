//! The Configuration key catalogue (ADR-0058): the keys Settings writes ([`ConfigKey`]), each in
//! its page's [`Section`], and a typed [`ConfigValue`] per key. Here so `lib-config`, which reads
//! and writes the file, and `lib-settings`, whose Presenters may not depend on it, name the same
//! keys and values.
//!
//! Only the catalogue lives here. How a key is spelt in the file, read from a parsed
//! Configuration and set from the environment is `lib-config`'s, beside its `KEYS` table, whose
//! tests fail until the two agree.

use std::path::PathBuf;

use crate::{ColourAppearance, DateStyle, Levels, OnStart, RowDensity, StartView, StatusGlyphs};

/// The sections Settings writes, one per THIS DEVICE page with keys (ADR-0057). `[Theme*]`,
/// `[Keybindings]` and `[Sync-Server]` are edited only by hand, so none is here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    /// `[General]`: Settings › General.
    General,
    /// `[Display]`: Settings › Display.
    Display,
    /// `[Logs]`: Settings › Logs.
    Logs,
}

impl Section {
    /// Every section Settings writes.
    pub const ALL: [Section; 3] = [Self::General, Self::Display, Self::Logs];

    /// The section's header as a write adds it to a file that lacks it.
    pub fn header(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Display => "Display",
            Self::Logs => "Logs",
        }
    }
}

/// The keys Settings writes, each in its page's [`Section`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ConfigKey {
    /// `on_start`: `reopen` or `none`.
    OnStart,
    /// `start_view`: `dashboard`, `last`, `settings` or a noun.
    StartView,
    /// `toasts`: whether non-Error Toasts show.
    Toasts,
    /// `locale`: the Locale's BCP-47 tag; removed for System, the operating system's.
    Locale,
    /// `data`: the folder `:open` and `:new` start in.
    Data,
    /// `colour_theme`: the Colour Theme's id.
    ColourTheme,
    /// `colour_appearance`: `light`, `dark` or `system`.
    ColourAppearance,
    /// `date_style`: `default`, `short`, `medium`, `long` or `iso`.
    DateStyle,
    /// `row_density`: `compact`, `regular` or `roomy`.
    RowDensity,
    /// `status_glyphs`: `unicode` or `ascii`.
    StatusGlyphs,
    /// `hint_bar`: whether the footer shows the key hints.
    HintBar,
    /// `terminal_colours`: draw with the terminal's own colours (ADR-0024).
    TerminalColours,
    /// `log`: the tracing level.
    Log,
    /// `log_file_path`: a file the logs are also written to.
    LogFilePath,
}

impl ConfigKey {
    /// Every key Settings writes, in the order the file's sections and keys are documented.
    pub const ALL: [ConfigKey; 14] = [
        Self::OnStart,
        Self::StartView,
        Self::Toasts,
        Self::Locale,
        Self::Data,
        Self::ColourTheme,
        Self::ColourAppearance,
        Self::DateStyle,
        Self::RowDensity,
        Self::StatusGlyphs,
        Self::HintBar,
        Self::TerminalColours,
        Self::Log,
        Self::LogFilePath,
    ];

    /// The key as it is written in the file.
    pub fn name(self) -> &'static str {
        match self {
            Self::OnStart => "on_start",
            Self::StartView => "start_view",
            Self::Toasts => "toasts",
            Self::Locale => "locale",
            Self::Data => "data",
            Self::ColourTheme => "colour_theme",
            Self::ColourAppearance => "colour_appearance",
            Self::DateStyle => "date_style",
            Self::RowDensity => "row_density",
            Self::StatusGlyphs => "status_glyphs",
            Self::HintBar => "hint_bar",
            Self::TerminalColours => "terminal_colours",
            Self::Log => "log",
            Self::LogFilePath => "log_file_path",
        }
    }

    /// The section the key lives in, its Settings page's.
    pub fn section(self) -> Section {
        match self {
            Self::OnStart | Self::StartView | Self::Toasts | Self::Locale | Self::Data => {
                Section::General
            }
            Self::ColourTheme
            | Self::ColourAppearance
            | Self::DateStyle
            | Self::RowDensity
            | Self::StatusGlyphs
            | Self::HintBar
            | Self::TerminalColours => Section::Display,
            Self::Log | Self::LogFilePath => Section::Logs,
        }
    }
}

/// A value Settings writes, typed by its key, so a value the parser would refuse can't be spelt:
/// the variant names the [`ConfigKey`] and holds the type the parser reads it into. The Locale and
/// the Colour Theme are the open-ended two: a Locale tag is checked as it is written, and a Colour
/// Theme id is kept raw, as the parser keeps it (ADR-0023).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigValue {
    OnStart(OnStart),
    StartView(StartView),
    Toasts(bool),
    /// A BCP-47 tag, since this crate can't name `lib_locale::Locale`; `lib_locale::Locale::choice`
    /// turns it into one. **System** is no value: the key is removed instead.
    Locale(String),
    Data(PathBuf),
    ColourTheme(String),
    ColourAppearance(ColourAppearance),
    /// `None` is the Locale's default, written `default`.
    DateStyle(Option<DateStyle>),
    RowDensity(RowDensity),
    StatusGlyphs(StatusGlyphs),
    HintBar(bool),
    TerminalColours(bool),
    Log(Levels),
    LogFilePath(PathBuf),
}

impl ConfigValue {
    /// The key the value is written under.
    pub fn key(&self) -> ConfigKey {
        match self {
            Self::OnStart(_) => ConfigKey::OnStart,
            Self::StartView(_) => ConfigKey::StartView,
            Self::Toasts(_) => ConfigKey::Toasts,
            Self::Locale(_) => ConfigKey::Locale,
            Self::Data(_) => ConfigKey::Data,
            Self::ColourTheme(_) => ConfigKey::ColourTheme,
            Self::ColourAppearance(_) => ConfigKey::ColourAppearance,
            Self::DateStyle(_) => ConfigKey::DateStyle,
            Self::RowDensity(_) => ConfigKey::RowDensity,
            Self::StatusGlyphs(_) => ConfigKey::StatusGlyphs,
            Self::HintBar(_) => ConfigKey::HintBar,
            Self::TerminalColours(_) => ConfigKey::TerminalColours,
            Self::Log(_) => ConfigKey::Log,
            Self::LogFilePath(_) => ConfigKey::LogFilePath,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_value_names_its_key() {
        assert_eq!(ConfigValue::Toasts(true).key(), ConfigKey::Toasts);
        assert_eq!(
            ConfigValue::Locale("en-AU".to_string()).key(),
            ConfigKey::Locale
        );
        assert_eq!(ConfigValue::Log(Levels::INFO).key(), ConfigKey::Log);
    }
}
