//! `impl DeviceConfiguration` for the Desktop (ADR-0053): Toasts, the Colour Theme and
//! Appearance, the date style, Row density, Status glyphs, On start, Start view, the Locale and the Data folder live in the
//! `[General]` and `[Display]` sections (ADR-0057) of the user's `personal-ledger.conf`, which Settings › General and Display write through
//! `lib_config`'s [`ConfigWriter`] as each one changes, the TUI's file and spellings included.
//! The pages' Presenters (`lib-settings`) decide what a write's answer means; this only writes.

use std::path::{Path, PathBuf};

use lib_colour_theme::ColourAppearance;
use lib_config::{ConfigKey, ConfigValue, ConfigWriter};
use lib_core::{DateStyle, OnStart, RowDensity, StartView, StatusGlyphs};
use lib_locale::Locale;
use lib_settings::{DeviceAnswer, DeviceConfiguration, DeviceKey, DeviceRefusal};

/// The file's key a device's setting is written under: the one per-key table here, read both
/// ways, since `lib-settings` can't name `lib-config`'s keys and `lib-config` spells every value.
fn config_key(key: DeviceKey) -> ConfigKey {
    match key {
        DeviceKey::OnStart => ConfigKey::OnStart,
        DeviceKey::StartView => ConfigKey::StartView,
        DeviceKey::Toasts => ConfigKey::Toasts,
        DeviceKey::DateStyle => ConfigKey::DateStyle,
        DeviceKey::RowDensity => ConfigKey::RowDensity,
        DeviceKey::StatusGlyphs => ConfigKey::StatusGlyphs,
        DeviceKey::ColourTheme => ConfigKey::ColourTheme,
        DeviceKey::ColourAppearance => ConfigKey::ColourAppearance,
        DeviceKey::Locale => ConfigKey::Locale,
        DeviceKey::Data => ConfigKey::Data,
    }
}

/// Where a device's settings are written and what outranks the file, as start-up's parse
/// recorded it in the writer's overrides report. The default has no file, so every write is
/// refused, as with no home folder.
#[derive(Debug, Clone, Default)]
pub struct DeviceConfigFile {
    writer: ConfigWriter,
}

impl DeviceConfigFile {
    pub fn new(writer: ConfigWriter) -> Self {
        Self { writer }
    }

    /// Writes `value` under its key, answering with `answer` once saved.
    fn write<T>(&self, value: ConfigValue, answer: T) -> DeviceAnswer<T> {
        let written = value.key();
        let Some(key) = DeviceKey::ALL
            .into_iter()
            .find(|key| config_key(*key) == written)
        else {
            tracing::error!(key = written.name(), "A device's setting with no DeviceKey");
            return DeviceAnswer::Refused(DeviceRefusal::WriteFailed);
        };
        self.change(key, answer, |writer, _| writer.set(value))
    }

    /// Changes `key` in the file with `edit`, unless a source above sets a key that refuses then.
    fn change<T>(
        &self,
        key: DeviceKey,
        value: T,
        edit: impl FnOnce(&ConfigWriter, ConfigKey) -> Result<(), lib_config::Error>,
    ) -> DeviceAnswer<T> {
        if !key.writes_when_shadowed()
            && let Some(source) = self.shadow(key)
        {
            return DeviceAnswer::Shadowed { source };
        }
        let Some(path) = self.writer.target() else {
            return DeviceAnswer::Refused(DeviceRefusal::NoFile);
        };
        match edit(&self.writer, config_key(key)) {
            Ok(()) => DeviceAnswer::Saved {
                value,
                file: path.display().to_string(),
            },
            Err(error) => {
                tracing::error!(%error, key = key.name(), "Couldn't write a device's setting");
                DeviceAnswer::Refused(DeviceRefusal::WriteFailed)
            }
        }
    }
}

impl DeviceConfiguration for DeviceConfigFile {
    fn shadow(&self, key: DeviceKey) -> Option<String> {
        self.writer
            .overrides()
            .get(config_key(key))
            .map(lib_config::Shadow::source)
    }

    fn set_on_start(&self, on_start: OnStart) -> DeviceAnswer<OnStart> {
        self.write(ConfigValue::OnStart(on_start), on_start)
    }

    fn set_start_view(&self, start_view: StartView) -> DeviceAnswer<StartView> {
        self.write(ConfigValue::StartView(start_view.clone()), start_view)
    }

    fn set_toasts(&self, on: bool) -> DeviceAnswer<bool> {
        self.write(ConfigValue::Toasts(on), on)
    }

    fn set_date_style(&self, style: Option<DateStyle>) -> DeviceAnswer<Option<DateStyle>> {
        self.write(ConfigValue::DateStyle(style), style)
    }

    fn set_row_density(&self, density: RowDensity) -> DeviceAnswer<RowDensity> {
        self.write(ConfigValue::RowDensity(density), density)
    }

    fn set_status_glyphs(&self, glyphs: StatusGlyphs) -> DeviceAnswer<StatusGlyphs> {
        self.write(ConfigValue::StatusGlyphs(glyphs), glyphs)
    }

    fn set_colour_theme(&self, id: &'static str) -> DeviceAnswer<&'static str> {
        self.write(ConfigValue::ColourTheme(id.to_string()), id)
    }

    fn set_colour_appearance(
        &self,
        appearance: ColourAppearance,
    ) -> DeviceAnswer<ColourAppearance> {
        self.write(ConfigValue::ColourAppearance(appearance), appearance)
    }

    /// **System** takes the key out, so the sources below it, the operating system's Locale
    /// last, decide at the next start.
    fn set_locale(&self, locale: Option<Locale>) -> DeviceAnswer<Option<Locale>> {
        match locale {
            Some(chosen) => self.write(ConfigValue::Locale(chosen.tag().to_string()), locale),
            None => self.change(DeviceKey::Locale, None, |writer, key| writer.remove(key)),
        }
    }

    fn set_data(&self, folder: &Path) -> DeviceAnswer<PathBuf> {
        self.write(
            ConfigValue::Data(folder.to_path_buf()),
            folder.to_path_buf(),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use lib_settings::contract::{self, DeviceFactory};
    use lib_settings::fake::StoredDevice;

    use super::*;

    fn writing_to(path: &Path, env: &[(String, String)]) -> DeviceConfigFile {
        DeviceConfigFile::new(ConfigWriter::new(
            Some(path.to_path_buf()),
            lib_config::Overrides::environment(env.iter().cloned()),
        ))
    }

    /// Each check's device on a file of its own in one temporary folder.
    struct ConfFiles {
        dir: tempfile::TempDir,
        made: usize,
    }

    impl ConfFiles {
        fn next_path(&mut self) -> PathBuf {
            self.made += 1;
            self.dir
                .path()
                .join(format!("{}/personal-ledger.conf", self.made))
        }
    }

    impl DeviceFactory for ConfFiles {
        type Device = DeviceConfigFile;

        fn writable(&mut self) -> Self::Device {
            writing_to(&self.next_path(), &[])
        }

        fn shadowing(&mut self, key: DeviceKey) -> (Self::Device, String) {
            let var = lib_config::env_var(config_key(key));
            let path = self.next_path();
            (writing_to(&path, &[(var.clone(), "x".to_string())]), var)
        }

        fn without_file(&mut self) -> Self::Device {
            DeviceConfigFile::default()
        }

        /// A file whose folder is a file, so nothing can be written under it.
        fn unwritable(&mut self) -> Self::Device {
            let blocker = self.dir.path().join("blocker");
            std::fs::write(&blocker, "").expect("a file to block the folder");
            writing_to(&blocker.join("personal-ledger.conf"), &[])
        }

        /// Reads the file back through `lib-config`'s parser, each key from its own section.
        fn reload(&mut self, device: &Self::Device) -> StoredDevice {
            let mut stored = StoredDevice::default();
            for key in DeviceKey::ALL {
                match device.writer.stored(config_key(key)) {
                    Some(ConfigValue::OnStart(value)) => stored.on_start = Some(value),
                    Some(ConfigValue::StartView(value)) => stored.start_view = Some(value),
                    Some(ConfigValue::Toasts(value)) => stored.toasts = Some(value),
                    Some(ConfigValue::DateStyle(value)) => stored.date_style = Some(value),
                    Some(ConfigValue::RowDensity(value)) => stored.row_density = Some(value),
                    Some(ConfigValue::StatusGlyphs(value)) => stored.status_glyphs = Some(value),
                    Some(ConfigValue::ColourTheme(value)) => stored.colour_theme = Some(value),
                    Some(ConfigValue::ColourAppearance(value)) => {
                        stored.colour_appearance = Some(value);
                    }
                    Some(ConfigValue::Locale(tag)) => {
                        stored.locale = Locale::choice(&tag);
                    }
                    Some(ConfigValue::Data(value)) => stored.data = Some(value),
                    _ => {}
                }
            }
            stored
        }
    }

    #[test]
    fn the_conf_file_keeps_the_device_configuration_contract() {
        let mut files = ConfFiles {
            dir: tempfile::tempdir().expect("a temporary directory"),
            made: 0,
        };
        contract::run_all(&mut files);
    }

    #[test]
    fn each_key_is_the_one_lib_config_writes() {
        for key in DeviceKey::ALL {
            assert_eq!(key.name(), config_key(key).name());
        }
    }

    #[test]
    fn each_setting_is_written_as_the_tuis_set_spells_it() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("personal-ledger.conf");
        let device = writing_to(&path, &[]);
        device.set_toasts(false);
        device.set_colour_theme("nord");
        device.set_colour_appearance(ColourAppearance::Light);
        device.set_date_style(None);
        device.set_on_start(OnStart::NoLedger);
        device.set_start_view(StartView::Noun("bills".into()));
        device.set_row_density(RowDensity::Compact);
        device.set_status_glyphs(StatusGlyphs::AsciiFallback);
        device.set_locale(Some(Locale::EnGb));
        assert_eq!(
            std::fs::read_to_string(&path).unwrap_or_default(),
            "[General]\ntoasts = false\non_start = none\nstart_view = bills\nlocale = en-GB\n\n\
             [Display]\ncolour_theme = nord\ncolour_appearance = light\ndate_style = default\n\
             row_density = compact\nstatus_glyphs = ascii\n"
        );
    }
}
