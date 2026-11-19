use lib_core::configuration::ConfigValue;

use super::{outcome, recorded};
use crate::device::DevicePorts;
use crate::protocols::DeviceAnswer;

/// Write a This device setting to the configuration file, spelt as the parser reads it back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteSetting(pub ConfigValue);

impl WriteSetting {
    pub fn run(self, ports: &DevicePorts) -> DeviceAnswer<ConfigValue> {
        let key = self.0.key();
        let written = self.0.clone();
        let answer = recorded(ports, key, self.0, |file| file.set(written));
        tracing::info!(
            key = key.name(),
            outcome = outcome(&answer),
            "Wrote a device setting"
        );
        answer
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use lib_core::RowDensity;
    use lib_core::configuration::ConfigKey;

    use super::*;
    use crate::device::DeviceFile;
    use crate::fake::{InMemoryDeviceFile, JournalFake};
    use crate::protocols::DeviceRefusal;

    fn ports(file: InMemoryDeviceFile) -> (Rc<InMemoryDeviceFile>, DevicePorts) {
        let file = Rc::new(file);
        (file.clone(), DevicePorts::new(file))
    }

    #[test]
    fn a_write_saves_the_value_and_names_the_file() {
        let (file, ports) = ports(InMemoryDeviceFile::writable());

        let answer = WriteSetting(ConfigValue::RowDensity(RowDensity::Compact)).run(&ports);

        assert_eq!(
            answer,
            DeviceAnswer::Saved {
                value: ConfigValue::RowDensity(RowDensity::Compact),
                file: "personal-ledger.conf".to_string()
            }
        );
        assert_eq!(
            file.stored().get(ConfigKey::RowDensity),
            Some(&ConfigValue::RowDensity(RowDensity::Compact))
        );
    }

    #[test]
    fn a_source_above_the_file_refuses_the_write_and_leaves_it_alone() {
        let (file, ports) = ports(InMemoryDeviceFile::shadowing(
            ConfigKey::RowDensity,
            "--density",
        ));

        let answer = WriteSetting(ConfigValue::RowDensity(RowDensity::Compact)).run(&ports);

        assert_eq!(
            answer,
            DeviceAnswer::Shadowed {
                source: "--density".to_string()
            }
        );
        assert_eq!(file.stored().get(ConfigKey::RowDensity), None);
    }

    #[test]
    fn a_key_that_writes_when_shadowed_is_written_under_its_override() {
        let (file, ports) = ports(InMemoryDeviceFile::shadowing(ConfigKey::Locale, "--locale"));

        let answer = WriteSetting(ConfigValue::Locale("en-GB".to_string())).run(&ports);

        assert!(matches!(answer, DeviceAnswer::Saved { .. }));
        assert!(file.stored().get(ConfigKey::Locale).is_some());
    }

    #[test]
    fn no_file_refuses_the_write() {
        let (_, ports) = ports(InMemoryDeviceFile::default());

        assert_eq!(
            WriteSetting(ConfigValue::Toasts(false)).run(&ports),
            DeviceAnswer::Refused(DeviceRefusal::NoFile)
        );
    }

    #[test]
    fn a_shadow_outranks_no_file() {
        let mut file = InMemoryDeviceFile::shadowing(ConfigKey::Toasts, "--toasts");
        file.file = None;
        let (_, ports) = ports(file);

        assert!(matches!(
            WriteSetting(ConfigValue::Toasts(false)).run(&ports),
            DeviceAnswer::Shadowed { .. }
        ));
    }

    #[test]
    fn a_failed_write_is_refused() {
        let (file, ports) = ports(InMemoryDeviceFile::unwritable());

        assert_eq!(
            WriteSetting(ConfigValue::Toasts(false)).run(&ports),
            DeviceAnswer::Refused(DeviceRefusal::WriteFailed)
        );
        assert_eq!(file.stored(), Default::default());
    }

    #[test]
    fn a_saved_write_records_the_raw_value_before_and_after() {
        let (file, ports) =
            ports(InMemoryDeviceFile::writable().storing([ConfigValue::Toasts(true)]));
        let journal = Rc::new(JournalFake::default());
        let ports = ports.logging(journal.clone());
        let before = file.raw(ConfigKey::Toasts);

        WriteSetting(ConfigValue::Toasts(false)).run(&ports);

        assert_eq!(
            journal.recorded(),
            vec![(ConfigKey::Toasts, before, file.raw(ConfigKey::Toasts))]
        );
    }

    #[test]
    fn a_refused_write_records_nothing_but_a_write_under_an_override_does() {
        let journal = Rc::new(JournalFake::default());
        let (_, refused) = ports(InMemoryDeviceFile::unwritable());
        WriteSetting(ConfigValue::Toasts(false)).run(&refused.logging(journal.clone()));
        let (_, shadowed) = ports(InMemoryDeviceFile::shadowing(ConfigKey::Toasts, "--toasts"));
        WriteSetting(ConfigValue::Toasts(false)).run(&shadowed.logging(journal.clone()));
        assert_eq!(journal.recorded(), vec![]);

        let (_, overridden) = ports(InMemoryDeviceFile::shadowing(ConfigKey::Locale, "--locale"));
        WriteSetting(ConfigValue::Locale("en-GB".to_string()))
            .run(&overridden.logging(journal.clone()));
        assert_eq!(journal.recorded().len(), 1);
    }
}
