//! `impl DeviceConfiguration` for the Desktop (ADR-0053): Toasts, the Colour Theme and
//! Appearance, the date style, Row density, Status glyphs, On start and Start view live in the
//! `[Personal-Ledger]` section of
//! the user's `personal-ledger.conf`, which Settings › General and Display write through
//! `lib_config`'s [`ConfigWriter`] as each one changes, the TUI's file and spellings included.
//! The pages' Presenters (`lib-settings`) decide what a write's answer means; this only writes.

use lib_colour_theme::ColourAppearance;
use lib_config::{ConfigKey, ConfigWriter, Shadow};
use lib_core::{DateStyle, OnStart, RowDensity, StartView, StatusGlyphs};
use lib_settings::{DeviceAnswer, DeviceConfiguration, DeviceKey, DeviceRefusal};

/// The file a key is written under.
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
    }
}

/// Where a device's settings are written and what outranks the file, as start-up found them.
/// The default has no file, so every write is refused, as with no home folder.
#[derive(Debug, Clone, Default)]
pub struct DeviceConfigFile {
    writer: ConfigWriter,
    /// The keys a source above the file sets, read once: the files above don't change while the
    /// Desktop runs, and a draw shouldn't read them.
    shadows: Vec<(DeviceKey, Shadow)>,
}

impl DeviceConfigFile {
    pub fn new(writer: ConfigWriter) -> Self {
        let shadows = DeviceKey::ALL
            .into_iter()
            .filter_map(|key| writer.shadow(config_key(key)).map(|shadow| (key, shadow)))
            .collect();
        Self { writer, shadows }
    }

    /// Writes `value`, spelt as the file and the TUI's `:set` spell it, under `key`.
    fn write<T>(&self, key: DeviceKey, spelt: &str, value: T) -> DeviceAnswer<T> {
        if let Some(source) = self.shadow(key) {
            return DeviceAnswer::Shadowed { source };
        }
        let Some(path) = self.writer.target() else {
            return DeviceAnswer::Refused(DeviceRefusal::NoFile);
        };
        match self.writer.write(config_key(key), spelt) {
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
        self.shadows
            .iter()
            .find(|(shadowed, _)| *shadowed == key)
            .map(|(_, shadow)| match shadow {
                Shadow::Env(var) => var.clone(),
                Shadow::File(path) => path.display().to_string(),
            })
    }

    fn set_on_start(&self, on_start: OnStart) -> DeviceAnswer<OnStart> {
        self.write(
            DeviceKey::OnStart,
            lib_config::on_start_key(on_start),
            on_start,
        )
    }

    fn set_start_view(&self, start_view: StartView) -> DeviceAnswer<StartView> {
        let spelt = start_view.key().to_string();
        self.write(DeviceKey::StartView, &spelt, start_view)
    }

    fn set_toasts(&self, on: bool) -> DeviceAnswer<bool> {
        self.write(DeviceKey::Toasts, if on { "true" } else { "false" }, on)
    }

    fn set_date_style(&self, style: Option<DateStyle>) -> DeviceAnswer<Option<DateStyle>> {
        self.write(
            DeviceKey::DateStyle,
            lib_config::date_style_key(style),
            style,
        )
    }

    fn set_row_density(&self, density: RowDensity) -> DeviceAnswer<RowDensity> {
        self.write(DeviceKey::RowDensity, density.key(), density)
    }

    fn set_status_glyphs(&self, glyphs: StatusGlyphs) -> DeviceAnswer<StatusGlyphs> {
        self.write(DeviceKey::StatusGlyphs, glyphs.key(), glyphs)
    }

    fn set_colour_theme(&self, id: &'static str) -> DeviceAnswer<&'static str> {
        self.write(DeviceKey::ColourTheme, id, id)
    }

    fn set_colour_appearance(
        &self,
        appearance: ColourAppearance,
    ) -> DeviceAnswer<ColourAppearance> {
        self.write(DeviceKey::ColourAppearance, appearance.key(), appearance)
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
            Vec::new(),
            env.iter().cloned(),
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
            let var = config_key(key).env_var();
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

        fn reload(&mut self, device: &Self::Device) -> StoredDevice {
            let text = device
                .writer
                .target()
                .and_then(|path| std::fs::read_to_string(path).ok())
                .unwrap_or_default();
            stored(&text)
        }
    }

    /// The `[Personal-Ledger]` keys `text` holds, read with the file's spellings.
    fn stored(text: &str) -> StoredDevice {
        let mut stored = StoredDevice::default();
        for (key, value) in text
            .lines()
            .filter_map(|line| line.split_once(" = "))
            .map(|(key, value)| (key.trim(), value.trim()))
        {
            match key {
                "on_start" => stored.on_start = lib_config::on_start_from_key(value),
                "start_view" => stored.start_view = Some(StartView::from_key(value)),
                "toasts" => stored.toasts = value.parse().ok(),
                "date_style" => {
                    stored.date_style = lib_settings::presenter::display::DATE_STYLE_CHOICES
                        .into_iter()
                        .find(|style| lib_config::date_style_key(*style) == value);
                }
                "row_density" => stored.row_density = RowDensity::from_key(value),
                "status_glyphs" => stored.status_glyphs = StatusGlyphs::from_key(value),
                "colour_theme" => stored.colour_theme = Some(value.to_string()),
                "colour_appearance" => {
                    stored.colour_appearance = ColourAppearance::from_key(value);
                }
                _ => {}
            }
        }
        stored
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
        assert_eq!(
            std::fs::read_to_string(&path).unwrap_or_default(),
            "[Personal-Ledger]\ntoasts = false\ncolour_theme = nord\ncolour_appearance = light\n\
             date_style = default\non_start = none\nstart_view = bills\nrow_density = compact\n\
             status_glyphs = ascii\n"
        );
    }
}
