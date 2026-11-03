//! A device's settings as Configuration (ADR-0053): Toasts, the Colour Theme and Appearance, the
//! date style, On start and Start view live in the `[Personal-Ledger]` section of the user's
//! `personal-ledger.conf`, which the This device Settings write through `lib_config`'s
//! [`ConfigWriter`] as each one changes, the TUI's file and spellings included. `Root` saves a
//! change before applying it, so a refused save leaves the setting as it was.

use std::path::PathBuf;

use gpui::{App, Global, prelude::*};
use lib_colour_theme::ColourAppearance;
use lib_config::{ConfigKey, ConfigWriter, Shadow};
use lib_core::{DateStyle, OnStart};

use crate::theme::colours::ColourChange;

/// One change to a device's setting, as Settings or the palette asks for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceSetting {
    Toasts(bool),
    /// A built-in Colour Theme's id.
    ColourTheme(&'static str),
    ColourAppearance(ColourAppearance),
    /// `None` is the Locale's default.
    DateStyle(Option<DateStyle>),
    OnStart(OnStart),
    /// One of [`crate::start_view::CHOICES`], as the file spells it.
    StartView(&'static str),
}

impl DeviceSetting {
    /// The six keys, for the shadows start-up looks for.
    const KEYS: [ConfigKey; 6] = [
        ConfigKey::Toasts,
        ConfigKey::ColourTheme,
        ConfigKey::ColourAppearance,
        ConfigKey::DateStyle,
        ConfigKey::OnStart,
        ConfigKey::StartView,
    ];

    pub fn key(self) -> ConfigKey {
        match self {
            Self::Toasts(_) => ConfigKey::Toasts,
            Self::ColourTheme(_) => ConfigKey::ColourTheme,
            Self::ColourAppearance(_) => ConfigKey::ColourAppearance,
            Self::DateStyle(_) => ConfigKey::DateStyle,
            Self::OnStart(_) => ConfigKey::OnStart,
            Self::StartView(_) => ConfigKey::StartView,
        }
    }

    /// The value as the file spells it, which is what the TUI's `:set` takes.
    fn file_value(self) -> &'static str {
        match self {
            Self::Toasts(true) => "true",
            Self::Toasts(false) => "false",
            Self::ColourTheme(id) => id,
            Self::ColourAppearance(appearance) => appearance.key(),
            Self::DateStyle(style) => lib_config::date_style_key(style),
            Self::OnStart(on_start) => lib_config::on_start_key(on_start),
            Self::StartView(key) => key,
        }
    }
}

impl From<ColourChange> for DeviceSetting {
    fn from(change: ColourChange) -> Self {
        match change {
            ColourChange::Theme(id) => Self::ColourTheme(id),
            ColourChange::Appearance(appearance) => Self::ColourAppearance(appearance),
        }
    }
}

/// Where a device's settings are written and what outranks the file, as start-up found them. A
/// Global, so a Settings page reads a row's shadowed note as it draws. With none installed there
/// is no file to write, and every save is refused.
#[derive(Debug, Clone, Default)]
pub struct DeviceConfiguration {
    writer: ConfigWriter,
    /// The keys a source above the file sets, read once: the files above don't change while the
    /// Desktop runs, and a draw shouldn't read them.
    shadows: Vec<(ConfigKey, Shadow)>,
}

impl Global for DeviceConfiguration {}

impl DeviceConfiguration {
    pub fn new(writer: ConfigWriter) -> Self {
        let shadows = DeviceSetting::KEYS
            .into_iter()
            .filter_map(|key| writer.shadow(key).map(|shadow| (key, shadow)))
            .collect();
        Self { writer, shadows }
    }

    /// What sets `key` above the file, by name, or `None` when a write to the file applies.
    fn shadow(&self, key: ConfigKey) -> Option<String> {
        self.shadows
            .iter()
            .find(|(shadowed, _)| *shadowed == key)
            .map(|(_, shadow)| match shadow {
                Shadow::Env(var) => var.clone(),
                Shadow::File(path) => path.display().to_string(),
            })
    }

    /// Writes `setting` to the file, returning the file it went to.
    ///
    /// # Errors
    /// A [`Refusal`] when a source above the file sets its key, there is no file to write, or the
    /// write failed, which is logged with its cause.
    pub fn save(&self, setting: DeviceSetting) -> Result<PathBuf, Refusal> {
        let key = setting.key();
        if let Some(source) = self.shadow(key) {
            return Err(Refusal::Shadowed { key, source });
        }
        let Some(path) = self.writer.target() else {
            return Err(Refusal::NoFile(key));
        };
        match self.writer.write(key, setting.file_value()) {
            Ok(()) => Ok(path.to_path_buf()),
            Err(error) => {
                tracing::error!(%error, key = key.name(), "Couldn't write a device's setting");
                Err(Refusal::WriteFailed(key))
            }
        }
    }
}

/// Why a device's setting wasn't saved, which the status line says. Not the crate's `Error`: a
/// shadowed key or a missing file is an answer for the person, not a failure for the caller to pass
/// on, and a failed write's cause is logged rather than spliced into the sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// A source above the file sets the key, named by `source`.
    Shadowed { key: ConfigKey, source: String },
    /// There is no home folder to keep the file in.
    NoFile(ConfigKey),
    /// The file or its folder couldn't be written.
    WriteFailed(ConfigKey),
}

impl Refusal {
    /// The status line's text.
    pub fn message(&self) -> String {
        match self {
            Self::Shadowed { key, source } => {
                crate::msg::desktop_settings_device_shadowed(key.name(), source)
            }
            Self::NoFile(key) => crate::msg::desktop_settings_device_no_file(key.name()),
            Self::WriteFailed(key) => crate::msg::desktop_settings_device_write_failed(key.name()),
        }
    }
}

/// Saves `setting` through the installed [`DeviceConfiguration`].
///
/// # Errors
/// As [`DeviceConfiguration::save`]; with none installed, there is no file to write.
pub fn save(setting: DeviceSetting, cx: &App) -> Result<PathBuf, Refusal> {
    cx.try_global::<DeviceConfiguration>().map_or_else(
        || DeviceConfiguration::default().save(setting),
        |device| device.save(setting),
    )
}

/// A control's note when a source above the file sets `key`, so the control says why a change
/// won't take. Drawn as both pages draw a control's hint.
pub fn shadowed_note(key: ConfigKey, cx: &App) -> Option<gpui::Div> {
    let source = cx.try_global::<DeviceConfiguration>()?.shadow(key)?;
    Some(
        gpui::div()
            .debug_selector(move || format!("shadowed-{}", key.name()))
            .mt(gpui::px(3.0))
            .text_size(gpui::px(12.0))
            .text_color(crate::theme::color::muted(cx))
            .child(crate::msg::desktop_settings_device_shadowed_note(&source)),
    )
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn writing_to(path: &Path, env: &[(&str, &str)]) -> DeviceConfiguration {
        DeviceConfiguration::new(ConfigWriter::new(
            Some(path.to_path_buf()),
            Vec::new(),
            env.iter()
                .map(|(name, value)| ((*name).to_string(), (*value).to_string())),
        ))
    }

    fn read(path: &Path) -> String {
        std::fs::read_to_string(path).unwrap_or_default()
    }

    #[test]
    fn each_setting_is_written_as_the_tuis_set_spells_it() {
        crate::locale::init_for_tests();
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("personal-ledger.conf");
        let device = writing_to(&path, &[]);
        for setting in [
            DeviceSetting::Toasts(false),
            DeviceSetting::ColourTheme("nord"),
            DeviceSetting::ColourAppearance(ColourAppearance::Light),
            DeviceSetting::DateStyle(None),
            DeviceSetting::OnStart(OnStart::NoLedger),
            DeviceSetting::StartView("bills"),
        ] {
            device.save(setting).expect("saved");
        }
        assert_eq!(
            read(&path),
            "[Personal-Ledger]\ntoasts = false\ncolour_theme = nord\ncolour_appearance = light\n\
             date_style = default\non_start = none\nstart_view = bills\n"
        );
        device
            .save(DeviceSetting::DateStyle(Some(DateStyle::Iso)))
            .expect("saved");
        assert!(read(&path).contains("date_style = iso\n"));
    }

    #[test]
    fn a_key_an_environment_variable_sets_is_refused_and_not_written() {
        crate::locale::init_for_tests();
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("personal-ledger.conf");
        let var = "PERSONAL_LEDGER_PERSONAL_LEDGER__TOASTS";
        let device = writing_to(&path, &[(var, "true")]);

        let refusal = device
            .save(DeviceSetting::Toasts(false))
            .expect_err("refused");

        assert!(refusal.message().contains(var), "{refusal:?}");
        assert_eq!(read(&path), "");
        assert_eq!(device.shadow(ConfigKey::Toasts).as_deref(), Some(var));
        assert_eq!(device.shadow(ConfigKey::OnStart), None);
    }

    #[test]
    fn with_no_file_to_write_a_save_is_refused() {
        crate::locale::init_for_tests();
        let refusal = DeviceConfiguration::default()
            .save(DeviceSetting::OnStart(OnStart::NoLedger))
            .expect_err("refused");
        assert_eq!(refusal, Refusal::NoFile(ConfigKey::OnStart));
        assert!(refusal.message().contains("on_start"));
    }
}
