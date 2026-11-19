use lib_core::configuration::ConfigKey;

use super::{change, outcome};
use crate::device::DevicePorts;
use crate::protocols::DeviceAnswer;

/// Put a This device setting back as the file held it, for Undo and Redo (#1244): `value` is its
/// raw text as read before or after a change, or `None` for a key the file didn't hold. Whatever
/// the file holds now is overwritten, and a value the parser refuses goes back as it was. Shadowed
/// as [`WriteSetting`](super::WriteSetting) is, and never recorded as a change of its own, so
/// stepping through History makes no entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreSetting {
    pub key: ConfigKey,
    pub value: Option<String>,
}

impl RestoreSetting {
    pub fn run(self, ports: &DevicePorts) -> DeviceAnswer<()> {
        let Self { key, value } = self;
        let answer = change(ports, key, (), |file| file.restore(key, value.as_deref()));
        tracing::info!(
            key = key.name(),
            outcome = outcome(&answer),
            "Restored a device setting"
        );
        answer
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use lib_core::configuration::ConfigValue;

    use super::*;
    use crate::fake::{InMemoryDeviceFile, JournalFake};
    use crate::protocols::DeviceRefusal;

    fn restore(key: ConfigKey, value: Option<&str>) -> RestoreSetting {
        RestoreSetting {
            key,
            value: value.map(str::to_string),
        }
    }

    #[test]
    fn a_restore_puts_the_raw_text_back_and_records_nothing() {
        let file = Rc::new(InMemoryDeviceFile::writable());
        let journal = Rc::new(JournalFake::default());
        let ports = DevicePorts::new(file.clone()).logging(journal.clone());

        let answer = restore(ConfigKey::StartView, Some("cash")).run(&ports);

        assert!(matches!(answer, DeviceAnswer::Saved { .. }));
        assert_eq!(
            file.restored(),
            vec![(ConfigKey::StartView, Some("cash".to_string()))]
        );
        assert_eq!(journal.recorded(), vec![]);
    }

    #[test]
    fn restoring_none_takes_the_key_out() {
        let file = Rc::new(InMemoryDeviceFile::writable().storing([ConfigValue::Toasts(false)]));

        restore(ConfigKey::Toasts, None).run(&DevicePorts::new(file.clone()));

        assert_eq!(file.stored().get(ConfigKey::Toasts), None);
    }

    #[test]
    fn a_shadowed_key_is_refused_unless_it_writes_when_shadowed() {
        let toasts = DevicePorts::new(Rc::new(InMemoryDeviceFile::shadowing(
            ConfigKey::Toasts,
            "--toasts",
        )));
        let locale = DevicePorts::new(Rc::new(InMemoryDeviceFile::shadowing(
            ConfigKey::Locale,
            "--locale",
        )));

        assert!(matches!(
            restore(ConfigKey::Toasts, Some("true")).run(&toasts),
            DeviceAnswer::Shadowed { .. }
        ));
        assert!(matches!(
            restore(ConfigKey::Locale, Some("en-GB")).run(&locale),
            DeviceAnswer::Saved { .. }
        ));
    }

    #[test]
    fn no_file_and_a_failed_write_are_refused() {
        for (file, refusal) in [
            (InMemoryDeviceFile::default(), DeviceRefusal::NoFile),
            (InMemoryDeviceFile::unwritable(), DeviceRefusal::WriteFailed),
        ] {
            let ports = DevicePorts::new(Rc::new(file));
            assert_eq!(
                restore(ConfigKey::Toasts, None).run(&ports),
                DeviceAnswer::Refused(refusal)
            );
        }
    }
}
