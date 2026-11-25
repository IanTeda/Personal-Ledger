use lib_core::configuration::ConfigKey;

use super::RestoreSetting;
use crate::device::DevicePorts;
use crate::protocols::DeviceAnswer;

/// Put a batch of This device settings back for one Undo or Redo (#1449), in order, each as
/// [`RestoreSetting`] does. The first key that doesn't go back, shadowed or refused, stops the
/// batch and the keys after it are left alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestoreSettings {
    pub changes: Vec<(ConfigKey, Option<String>)>,
}

/// How a [`RestoreSettings`] went: how many keys went back, and the first one that didn't, with
/// its answer (never `Saved`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Restored {
    pub restored: usize,
    pub refusal: Option<(ConfigKey, DeviceAnswer<()>)>,
}

impl RestoreSettings {
    pub fn run(self, ports: &DevicePorts) -> Restored {
        let mut restored = 0;
        let mut refusal = None;
        for (key, value) in self.changes {
            match (RestoreSetting { key, value }).run(ports) {
                DeviceAnswer::Saved { .. } => restored += 1,
                answer => {
                    refusal = Some((key, answer));
                    break;
                }
            }
        }
        tracing::info!(
            restored,
            refused = refusal.as_ref().map(|(key, _)| key.name()),
            "Restored device settings"
        );
        Restored { restored, refusal }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::fake::InMemoryDeviceFile;

    #[test]
    fn every_key_goes_back_when_none_refuses() {
        let file = Rc::new(InMemoryDeviceFile::writable());
        let ports = DevicePorts::new(file.clone());

        let done = RestoreSettings {
            changes: vec![
                (ConfigKey::StartView, Some("cash".to_string())),
                (ConfigKey::Toasts, None),
            ],
        }
        .run(&ports);

        assert_eq!(done.restored, 2);
        assert_eq!(done.refusal, None);
        assert_eq!(file.restored().len(), 2);
    }

    #[test]
    fn a_refused_key_stops_the_batch() {
        let file = Rc::new(InMemoryDeviceFile::shadowing(ConfigKey::Toasts, "--toasts"));
        let ports = DevicePorts::new(file.clone());

        let done = RestoreSettings {
            changes: vec![
                (ConfigKey::StartView, Some("cash".to_string())),
                (ConfigKey::Toasts, Some("true".to_string())),
                (ConfigKey::Locale, Some("en-GB".to_string())),
            ],
        }
        .run(&ports);

        assert_eq!(done.restored, 1);
        assert!(matches!(
            done.refusal,
            Some((ConfigKey::Toasts, DeviceAnswer::Shadowed { .. }))
        ));
        assert_eq!(
            file.restored(),
            vec![(ConfigKey::StartView, Some("cash".to_string()))]
        );
    }
}
