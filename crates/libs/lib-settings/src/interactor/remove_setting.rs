use lib_core::configuration::ConfigKey;

use super::{outcome, recorded};
use crate::device::DevicePorts;
use crate::protocols::DeviceAnswer;

/// Take a This device setting out of the file, so the sources below it decide at the next start:
/// the Locale's **System**. Shadowed as [`WriteSetting`](super::WriteSetting) is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoveSetting(pub ConfigKey);

impl RemoveSetting {
    pub fn run(self, ports: &DevicePorts) -> DeviceAnswer<()> {
        let key = self.0;
        let answer = recorded(ports, key, (), |file| file.remove(key));
        tracing::info!(
            key = key.name(),
            outcome = outcome(&answer),
            "Removed a device setting"
        );
        answer
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use lib_core::configuration::ConfigValue;

    use super::*;
    use crate::device::DeviceFile;
    use crate::error::Error;
    use crate::fake::{InMemoryDeviceFile, JournalFake};

    #[test]
    fn a_removal_takes_the_key_out_and_a_repeat_is_still_saved() {
        let file = Rc::new(InMemoryDeviceFile::writable().storing([ConfigValue::Toasts(false)]));
        let ports = DevicePorts::new(file.clone());

        assert!(matches!(
            RemoveSetting(ConfigKey::Toasts).run(&ports),
            DeviceAnswer::Saved { .. }
        ));
        assert!(matches!(
            RemoveSetting(ConfigKey::Toasts).run(&ports),
            DeviceAnswer::Saved { .. }
        ));
        assert_eq!(file.stored().get(ConfigKey::Toasts), None);
    }

    #[test]
    fn a_shadowed_key_is_refused_and_kept() {
        let file = Rc::new(
            InMemoryDeviceFile::shadowing(ConfigKey::Toasts, "--toasts")
                .storing([ConfigValue::Toasts(false)]),
        );

        let answer = RemoveSetting(ConfigKey::Toasts).run(&DevicePorts::new(file.clone()));

        assert_eq!(
            answer,
            DeviceAnswer::Shadowed {
                source: "--toasts".to_string()
            }
        );
        assert!(file.stored().get(ConfigKey::Toasts).is_some());
    }

    #[test]
    fn no_file_and_a_failed_write_are_refused() {
        for (file, refusal) in [
            (InMemoryDeviceFile::default(), Error::NoFile),
            (InMemoryDeviceFile::unwritable(), Error::WriteFailed),
        ] {
            let ports = DevicePorts::new(Rc::new(file));
            assert_eq!(
                RemoveSetting(ConfigKey::Toasts).run(&ports),
                DeviceAnswer::Refused(refusal)
            );
        }
    }

    #[test]
    fn a_saved_removal_records_the_raw_value_before_and_none_after() {
        let file = Rc::new(InMemoryDeviceFile::writable().storing([ConfigValue::Toasts(false)]));
        let journal = Rc::new(JournalFake::default());
        let ports = DevicePorts::new(file.clone()).logging(journal.clone());
        let before = file.raw(ConfigKey::Toasts);

        RemoveSetting(ConfigKey::Toasts).run(&ports);

        assert_eq!(journal.recorded(), vec![(ConfigKey::Toasts, before, None)]);
    }
}
