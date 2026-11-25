//! What the Settings pages share beside the device settings form (`form.rs`): the stock note
//! under a field whose key a source above the file sets, and the status line's words for how a
//! write of a device's setting went.

use lib_core::configuration::ConfigKey;

use crate::error::Error;
use crate::msg;
use crate::protocols::DeviceAnswer;

/// The note under a control whose key `source`, a source above the file, sets; `None` for none.
pub(super) fn shadowed_note(source: Option<&str>) -> Option<String> {
    source.map(msg::settings_device_shadowed_note)
}

/// How a write or restore of `key` went, in the status line's words: the one wording a page's
/// write and an Undo or Redo of it share, so a refusal reads the same either way.
pub fn device_status<T>(answer: &DeviceAnswer<T>, key: ConfigKey) -> String {
    let name = key.name();
    match answer {
        DeviceAnswer::Saved { file, .. } => msg::settings_device_saved(name, file),
        DeviceAnswer::Shadowed { source } => {
            lib_locale::msg::settings_device_shadowed(name, source)
        }
        DeviceAnswer::Refused(Error::NoFile) => lib_locale::msg::settings_device_no_file(name),
        // A base Unit refusal never answers a device write; read it as the write failing.
        DeviceAnswer::Refused(Error::WriteFailed | Error::BaseUnitRefused) => {
            lib_locale::msg::settings_device_write_failed(name)
        }
    }
}

#[cfg(test)]
mod tests {
    use lib_core::configuration::ConfigKey;

    use super::*;
    use crate::error::Error;
    use crate::protocols::DeviceAnswer;

    #[test]
    fn a_saved_answer_names_the_key_and_the_file() {
        let answer = DeviceAnswer::Saved {
            value: (),
            file: "~/.config/ledger.conf".to_string(),
        };
        assert_eq!(
            device_status(&answer, ConfigKey::Toasts),
            msg::settings_device_saved(ConfigKey::Toasts.name(), "~/.config/ledger.conf")
        );
    }

    #[test]
    fn a_shadowed_answer_names_the_key_and_the_source_above_the_file() {
        let answer: DeviceAnswer<()> = DeviceAnswer::Shadowed {
            source: "PERSONAL_LEDGER_TOASTS".to_string(),
        };
        assert_eq!(
            device_status(&answer, ConfigKey::Toasts),
            lib_locale::msg::settings_device_shadowed(
                ConfigKey::Toasts.name(),
                "PERSONAL_LEDGER_TOASTS"
            )
        );
    }

    #[test]
    fn a_refusal_for_no_file_says_there_is_nowhere_to_keep_it() {
        let answer: DeviceAnswer<()> = DeviceAnswer::Refused(Error::NoFile);
        assert_eq!(
            device_status(&answer, ConfigKey::RowDensity),
            lib_locale::msg::settings_device_no_file(ConfigKey::RowDensity.name())
        );
    }

    #[test]
    fn a_refusal_for_a_failed_write_says_the_write_failed() {
        let answer: DeviceAnswer<()> = DeviceAnswer::Refused(Error::WriteFailed);
        assert_eq!(
            device_status(&answer, ConfigKey::RowDensity),
            lib_locale::msg::settings_device_write_failed(ConfigKey::RowDensity.name())
        );
    }
}
