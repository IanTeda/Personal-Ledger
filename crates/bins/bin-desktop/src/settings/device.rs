//! `impl DeviceFile` for the Desktop (ADR-0053): a device's settings live in the
//! `[General]`, `[Display]` and `[Logs]` sections (ADR-0057) of the user's `personal-ledger.conf`,
//! which Settings reads and writes through `lib_config`'s [`ConfigWriter`], the TUI's file and
//! spellings included. Nothing here is per key: the catalogue (`lib_core::configuration`) and
//! `lib_config` spell every key and value. The Interactors (`lib-settings`) decide whether a write goes
//! through and the pages' Presenters what its answer means; this only reads and writes.

use std::path::Path;
use std::rc::Rc;

use crate::reversal::{ConfigJournal, ConfigReversal};
use lib_config::ConfigWriter;
use lib_core::configuration::{ConfigKey, ConfigValue};
use lib_settings::{DeviceFile, DevicePorts, WriteError};

/// Where a device's settings are written, what outranks the file, as start-up's parse recorded it
/// in the writer's overrides report, and each key's value in force from that parse. The default
/// has no file. No policy lives here: the Interactors decide what a write may do.
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
}

/// Logs a failed write's cause, which the policy only sees as a [`WriteError`].
fn failed(key: ConfigKey) -> impl FnOnce(lib_config::Error) -> WriteError {
    move |error| {
        tracing::error!(%error, key = key.name(), "Couldn't write a device's setting");
        WriteError
    }
}

impl DeviceFile for DeviceConfigFile {
    fn target(&self) -> Option<&Path> {
        self.writer.target()
    }

    fn shadow(&self, key: ConfigKey) -> Option<String> {
        self.writer
            .overrides()
            .get(key)
            .map(lib_config::Shadow::source)
    }

    fn stored_all(&self) -> Vec<(ConfigKey, ConfigValue)> {
        self.writer.stored_all()
    }

    fn effective(&self, key: ConfigKey) -> Option<ConfigValue> {
        self.effective
            .iter()
            .find(|(read, _)| *read == key)
            .map(|(_, value)| value.clone())
    }

    fn raw(&self, key: ConfigKey) -> Option<String> {
        self.writer.value(key)
    }

    fn set(&self, value: ConfigValue) -> Result<(), WriteError> {
        let key = value.key();
        self.writer.set(value).map_err(failed(key))
    }

    /// The sources below the file, the operating system's Locale last, decide at the next start.
    fn remove(&self, key: ConfigKey) -> Result<(), WriteError> {
        self.writer.remove(key).map_err(failed(key))
    }

    fn restore(&self, key: ConfigKey, value: Option<&str>) -> Result<(), WriteError> {
        self.writer.restore(key, value).map_err(failed(key))
    }
}

/// The ports Settings' Interactors write the device through: the configuration file, recording each
/// saved change in the [`ConfigJournal`] the Shell's root Router takes each input's History entry
/// from, as `lib-database`'s journal is for the Ledger's rows.
pub struct ConfiguredDevice {
    ports: DevicePorts,
    /// What the running input changed in the file, shared with `Root`, whose `send` takes it.
    journal: ConfigJournal,
}

impl ConfiguredDevice {
    pub fn new(config: &lib_config::Config, writer: ConfigWriter) -> Self {
        Self::over(DeviceConfigFile::new(config, writer))
    }

    /// `file` journalling into a journal of its own.
    pub fn over(file: DeviceConfigFile) -> Self {
        let journal = ConfigJournal::default();
        Self {
            ports: DevicePorts::new(Rc::new(file)).logging(Rc::new(journal.clone())),
            journal,
        }
    }

    /// The ports the This device pages' Presenters write through.
    pub(crate) fn ports(&self) -> DevicePorts {
        self.ports.clone()
    }

    /// The journal this device records into, for the Shell's root Router to take each input's
    /// changes from.
    pub(crate) fn journal(&self) -> ConfigJournal {
        self.journal.clone()
    }

    /// The changes recorded since the last take, in the order written.
    pub(crate) fn take(&self) -> ConfigReversal {
        self.journal.take()
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use lib_core::{ColourAppearance, DateStyle, OnStart, RowDensity, StartView, StatusGlyphs};
    use lib_settings::contract::{self, DeviceFileFactory};
    use lib_settings::{DeviceAnswer, DeviceRefusal, RemoveSetting, RestoreSetting, WriteSetting};

    use super::*;
    use crate::reversal::ConfigChange;

    fn file_at(path: &Path, env: &[(String, String)]) -> DeviceConfigFile {
        DeviceConfigFile::new(
            &lib_config::Config::default(),
            ConfigWriter::new(
                Some(path.to_path_buf()),
                lib_config::Overrides::environment(env.iter().cloned()),
            ),
        )
    }

    fn restore(device: &ConfiguredDevice, key: ConfigKey, value: Option<&str>) -> DeviceAnswer<()> {
        RestoreSetting {
            key,
            value: value.map(str::to_string),
        }
        .run(&device.ports())
    }

    fn writing_to(path: &Path, env: &[(String, String)]) -> ConfiguredDevice {
        ConfiguredDevice::over(file_at(path, env))
    }

    /// Each check's file of its own in one temporary folder.
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

    impl DeviceFileFactory for ConfFiles {
        type File = DeviceConfigFile;

        fn writable(&mut self) -> Self::File {
            file_at(&self.next_path(), &[])
        }

        fn shadowing(&mut self, key: ConfigKey) -> (Self::File, String) {
            let var = lib_config::env_var(key);
            let path = self.next_path();
            (file_at(&path, &[(var.clone(), "x".to_string())]), var)
        }

        fn without_file(&mut self) -> Self::File {
            DeviceConfigFile::default()
        }

        /// A file whose folder is a file, so nothing can be written under it.
        fn unwritable(&mut self) -> Self::File {
            let blocker = self.dir.path().join("blocker");
            std::fs::write(&blocker, "").expect("a file to block the folder");
            file_at(&blocker.join("personal-ledger.conf"), &[])
        }
    }

    #[test]
    fn the_conf_file_keeps_the_device_file_contract() {
        let mut files = ConfFiles {
            dir: tempfile::tempdir().expect("a temporary directory"),
            made: 0,
        };
        contract::run_all_device_file(&mut files);
    }

    #[test]
    fn the_conf_file_keeps_the_write_through_policy() {
        let mut files = ConfFiles {
            dir: tempfile::tempdir().expect("a temporary directory"),
            made: 0,
        };
        contract::run_all_policy(&mut files);
    }

    #[test]
    fn the_value_in_force_is_start_ups_parse_whatever_the_file_holds() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("personal-ledger.conf");
        let config = lib_config::Config::default();
        let device = ConfiguredDevice::new(
            &config,
            ConfigWriter::new(Some(path.clone()), lib_config::Overrides::default()),
        );

        WriteSetting(ConfigValue::RowDensity(RowDensity::Roomy)).run(&device.ports());

        let snapshot = device.ports().snapshot();
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
            WriteSetting(value).run(&device.ports());
        }
        assert_eq!(
            std::fs::read_to_string(&path).unwrap_or_default(),
            "[General]\ntoasts = false\non_start = none\nstart_view = bills\nlocale = en-GB\n\n\
             [Display]\ncolour_theme = nord\ncolour_appearance = light\ndate_style = default\n\
             row_density = compact\nstatus_glyphs = ascii\n"
        );
    }

    #[test]
    fn a_write_records_the_files_raw_values_before_and_after_and_a_restore_records_nothing() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("personal-ledger.conf");
        std::fs::write(&path, "[General]\nstart_view = cash\n").expect("the file written");
        let device = writing_to(&path, &[]);

        WriteSetting(ConfigValue::StartView(StartView::Dashboard)).run(&device.ports());
        WriteSetting(ConfigValue::StartView(StartView::Dashboard)).run(&device.ports());
        RemoveSetting(ConfigKey::Locale).run(&device.ports());

        assert_eq!(
            device.take(),
            ConfigReversal::from(vec![ConfigChange {
                key: ConfigKey::StartView,
                before: Some("cash".to_string()),
                after: Some("dashboard".to_string()),
            }]),
            "a repeat and a removal of nothing change nothing"
        );

        assert!(matches!(
            restore(&device, ConfigKey::StartView, Some("cash")),
            DeviceAnswer::Saved { .. }
        ));
        assert_eq!(device.take(), ConfigReversal::default());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap_or_default(),
            "[General]\nstart_view = cash\n"
        );
    }

    #[test]
    fn a_shadowed_locale_restores_and_a_shadowed_toasts_is_refused() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let path = dir.path().join("personal-ledger.conf");
        let env = [
            (lib_config::env_var(ConfigKey::Locale), "en-US".to_string()),
            (lib_config::env_var(ConfigKey::Toasts), "false".to_string()),
        ];
        let device = writing_to(&path, &env);

        assert!(matches!(
            restore(&device, ConfigKey::Locale, Some("en-GB")),
            DeviceAnswer::Saved { .. }
        ));
        assert!(matches!(
            restore(&device, ConfigKey::Toasts, Some("true")),
            DeviceAnswer::Shadowed { .. }
        ));
        assert!(matches!(
            restore(
                &ConfiguredDevice::over(DeviceConfigFile::default()),
                ConfigKey::Toasts,
                None
            ),
            DeviceAnswer::Refused(DeviceRefusal::NoFile)
        ));
    }
}
