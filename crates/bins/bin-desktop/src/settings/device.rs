//! `impl DeviceConfiguration` for the Desktop (ADR-0053): a device's settings live in the
//! `[General]`, `[Display]` and `[Logs]` sections (ADR-0057) of the user's `personal-ledger.conf`,
//! which Settings reads and writes through `lib_config`'s [`ConfigWriter`], the TUI's file and
//! spellings included. Nothing here is per key: the catalogue (`lib_core::configuration`) and
//! `lib_config` spell every key and value. The pages' Presenters (`lib-settings`) decide what a
//! write's answer means; this only reads and writes.

use lib_config::{ConfigWriter, Shadow};
use lib_core::configuration::{ConfigKey, ConfigValue};
use lib_settings::{
    DeviceAnswer, DeviceConfiguration, DeviceRefusal, DeviceSnapshot, KeyState,
    writes_when_shadowed,
};

/// Where a device's settings are written, what outranks the file, as start-up's parse recorded it
/// in the writer's overrides report, and each key's value in force from that parse. The default
/// has no file, so every write is refused, as with no home folder.
#[derive(Debug, Clone, Default)]
pub struct DeviceConfigFile {
    writer: ConfigWriter,
    /// Each key's value in force, by [`ConfigKey::ALL`]'s order, read once from start-up's parse:
    /// a write changes the file, not this run.
    effective: Vec<(ConfigKey, ConfigValue)>,
}

impl DeviceConfigFile {
    pub fn new(config: &lib_config::Config, writer: ConfigWriter) -> Self {
        Self {
            writer,
            effective: ConfigKey::ALL
                .into_iter()
                .filter_map(|key| Some((key, lib_config::read_value(key, config)?)))
                .collect(),
        }
    }

    /// Changes `key` in the file with `edit`, answering `value` once saved, unless a source above
    /// sets a key that refuses then.
    fn change<T>(
        &self,
        key: ConfigKey,
        value: T,
        edit: impl FnOnce(&ConfigWriter) -> Result<(), lib_config::Error>,
    ) -> DeviceAnswer<T> {
        if !writes_when_shadowed(key)
            && let Some(shadow) = self.writer.overrides().get(key)
        {
            return DeviceAnswer::Shadowed {
                source: shadow.source(),
            };
        }
        let Some(path) = self.writer.target() else {
            return DeviceAnswer::Refused(DeviceRefusal::NoFile);
        };
        match edit(&self.writer) {
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
    fn read(&self) -> DeviceSnapshot {
        DeviceSnapshot::new(self.writer.target().map(Into::into), |key| KeyState {
            stored: self.writer.stored(key),
            effective: self
                .effective
                .iter()
                .find(|(read, _)| *read == key)
                .map(|(_, value)| value.clone()),
            shadow: self.writer.overrides().get(key).map(Shadow::source),
        })
    }

    fn write(&self, value: ConfigValue) -> DeviceAnswer<ConfigValue> {
        let written = value.clone();
        self.change(value.key(), value, |writer| writer.set(written))
    }

    /// The sources below the file, the operating system's Locale last, decide at the next start.
    fn remove(&self, key: ConfigKey) -> DeviceAnswer<()> {
        self.change(key, (), |writer| writer.remove(key))
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use lib_core::{ColourAppearance, DateStyle, OnStart, RowDensity, StartView, StatusGlyphs};
    use lib_settings::contract::{self, DeviceFactory};

    use super::*;

    fn writing_to(path: &Path, env: &[(String, String)]) -> DeviceConfigFile {
        DeviceConfigFile::new(
            &lib_config::Config::default(),
            ConfigWriter::new(
                Some(path.to_path_buf()),
                lib_config::Overrides::environment(env.iter().cloned()),
            ),
        )
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

        fn shadowing(&mut self, key: ConfigKey) -> (Self::Device, String) {
            let var = lib_config::env_var(key);
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
    fn the_value_in_force_is_start_ups_parse_whatever_the_file_holds() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("personal-ledger.conf");
        let config = lib_config::Config::default();
        let device = DeviceConfigFile::new(
            &config,
            ConfigWriter::new(Some(path.clone()), lib_config::Overrides::default()),
        );

        device.write(ConfigValue::RowDensity(RowDensity::Roomy));

        let snapshot = device.read();
        assert_eq!(snapshot.file, Some(path));
        for key in ConfigKey::ALL {
            assert_eq!(
                snapshot.get(key).effective,
                lib_config::read_value(key, &config),
                "{key:?}"
            );
        }
        assert_eq!(
            snapshot.get(ConfigKey::RowDensity).stored,
            Some(ConfigValue::RowDensity(RowDensity::Roomy))
        );
    }

    #[test]
    fn each_setting_is_written_as_the_tuis_set_spells_it() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("personal-ledger.conf");
        let device = writing_to(&path, &[]);
        for value in [
            ConfigValue::Toasts(false),
            ConfigValue::ColourTheme("nord".to_string()),
            ConfigValue::ColourAppearance(ColourAppearance::Light),
            ConfigValue::DateStyle(None::<DateStyle>),
            ConfigValue::OnStart(OnStart::NoLedger),
            ConfigValue::StartView(StartView::Noun("bills".into())),
            ConfigValue::RowDensity(RowDensity::Compact),
            ConfigValue::StatusGlyphs(StatusGlyphs::AsciiFallback),
            ConfigValue::Locale("en-GB".to_string()),
        ] {
            device.write(value);
        }
        assert_eq!(
            std::fs::read_to_string(&path).unwrap_or_default(),
            "[General]\ntoasts = false\non_start = none\nstart_view = bills\nlocale = en-GB\n\n\
             [Display]\ncolour_theme = nord\ncolour_appearance = light\ndate_style = default\n\
             row_density = compact\nstatus_glyphs = ascii\n"
        );
    }
}
