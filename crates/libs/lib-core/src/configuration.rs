//! The Configuration key catalogue (ADR-0058): the keys Settings writes ([`ConfigKey`]), each in
//! its page's [`Section`], and a typed [`ConfigValue`] per key. Here so `lib-config`, which reads
//! and writes the file, and `lib-settings`, whose Presenters may not depend on it, name the same
//! keys and values.
//!
//! A key is one row of the `config_keys!` table below, which generates every per-key arm here.
//! How a key is spelt in the file, read from a parsed Configuration and set from the environment
//! is `lib-config`'s, which derives the written sections of its key table, `keys()`, from [`ConfigKey::ALL`].

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
    /// `[Logs]`: Settings › Tracing.
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

/// The catalogue's one table: each row, `Variant => (Section, "file_name", ValueType)`, generates
/// the [`ConfigKey`] variant, its place in [`ConfigKey::ALL`], its [`ConfigKey::name`] and
/// [`ConfigKey::section`], and the [`ConfigValue`] variant holding `ValueType` with its
/// [`ConfigValue::key`]. A row's doc comment documents both variants. Rows are in the order the
/// file's sections and keys are documented, which is [`ConfigKey::ALL`]'s.
macro_rules! config_keys {
    ($(
        $(#[doc = $doc:literal])*
        $variant:ident => ($section:ident, $name:literal, $value:ty),
    )*) => {
        /// The keys Settings writes, each in its page's [`Section`].
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum ConfigKey {
            $(
                $(#[doc = $doc])*
                $variant,
            )*
        }

        impl ConfigKey {
            /// Every key Settings writes, in the order the file's sections and keys are
            /// documented.
            pub const ALL: [ConfigKey; [$(stringify!($variant)),*].len()] =
                [$(Self::$variant),*];

            /// The key as it is written in the file.
            pub fn name(self) -> &'static str {
                match self {
                    $(Self::$variant => $name,)*
                }
            }

            /// The section the key lives in, its Settings page's.
            pub fn section(self) -> Section {
                match self {
                    $(Self::$variant => Section::$section,)*
                }
            }
        }

        /// A value Settings writes, typed by its key, so a value the parser would refuse can't be
        /// spelt: the variant names the [`ConfigKey`] and holds the type the parser reads it into.
        /// The Locale and the Colour Theme are the open-ended two: a Locale tag is checked as it
        /// is written, and a Colour Theme id is kept raw, as the parser keeps it (ADR-0023).
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub enum ConfigValue {
            $(
                $(#[doc = $doc])*
                $variant($value),
            )*
        }

        impl ConfigValue {
            /// The key the value is written under.
            pub fn key(&self) -> ConfigKey {
                match self {
                    $(Self::$variant(_) => ConfigKey::$variant,)*
                }
            }
        }
    };
}

impl ConfigKey {
    /// Whether a write of the key goes to the file even while an Override sets it: the Locale and
    /// the Data folder (DESKTOP-186), whose choice is kept for when the Override is gone. Every
    /// other key refuses. A Settings rule, not the file's, so both Clients and every
    /// This device Interactor read the one answer.
    ///
    /// Exhaustive with no wildcard arm, so a new key won't compile until it decides.
    pub fn writes_when_shadowed(self) -> bool {
        match self {
            Self::Locale | Self::Data => true,
            Self::OnStart
            | Self::StartView
            | Self::Toasts
            | Self::ColourTheme
            | Self::ColourAppearance
            | Self::DateStyle
            | Self::RowDensity
            | Self::StatusGlyphs
            | Self::StartSidebarMinimised
            | Self::HintBar
            | Self::TerminalColours
            | Self::Log
            | Self::LogFilePath => false,
        }
    }
}

config_keys! {
    /// `on_start`: `reopen` or `none`.
    OnStart => (General, "on_start", OnStart),
    /// `start_view`: `dashboard`, `last`, `settings` or a noun.
    StartView => (General, "start_view", StartView),
    /// `toasts`: whether non-Error Toasts show.
    Toasts => (General, "toasts", bool),
    /// `locale`: the Locale's BCP-47 tag; removed for System, the operating system's. The value is
    /// the tag, since this crate can't name `lib_locale::Locale`; `lib_locale::Locale::choice`
    /// turns it into one. **System** is no value: the key is removed instead.
    Locale => (General, "locale", String),
    /// `data`: the folder `:open` and `:new` start in.
    Data => (General, "data", PathBuf),
    /// `colour_theme`: the Colour Theme's id.
    ColourTheme => (Display, "colour_theme", String),
    /// `colour_appearance`: `light`, `dark` or `system`.
    ColourAppearance => (Display, "colour_appearance", ColourAppearance),
    /// `date_style`: `default`, `short`, `medium`, `long` or `iso`. A `None` value is the Locale's
    /// default, written `default`.
    DateStyle => (Display, "date_style", Option<DateStyle>),
    /// `row_density`: `compact`, `regular` or `roomy`.
    RowDensity => (Display, "row_density", RowDensity),
    /// `status_glyphs`: `unicode` or `ascii`.
    StatusGlyphs => (Display, "status_glyphs", StatusGlyphs),
    /// `start_sidebar_minimised`: whether the Desktop starts with the primary rail collapsed.
    StartSidebarMinimised => (Display, "start_sidebar_minimised", bool),
    /// `hint_bar`: whether the footer shows the key hints.
    HintBar => (Display, "hint_bar", bool),
    /// `terminal_colours`: draw with the terminal's own colours (ADR-0024).
    TerminalColours => (Display, "terminal_colours", bool),
    /// `log`: the tracing level.
    Log => (Logs, "log", Levels),
    /// `log_file_path`: a file the logs are also written to.
    LogFilePath => (Logs, "log_file_path", PathBuf),
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

    #[test]
    fn only_the_locale_and_the_data_folder_are_written_while_shadowed() {
        let written: Vec<_> = ConfigKey::ALL
            .into_iter()
            .filter(|key| key.writes_when_shadowed())
            .collect();
        assert_eq!(written, [ConfigKey::Locale, ConfigKey::Data]);
    }
}
