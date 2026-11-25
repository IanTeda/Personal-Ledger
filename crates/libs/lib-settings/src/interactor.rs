//! One Interactor per This device intent, each a request value whose `run` takes the
//! [`DevicePorts`]. They share one policy, decided in one place: a source above the file that sets
//! the key refuses the change (except for a key that
//! [writes when shadowed](lib_core::configuration::ConfigKey::writes_when_shadowed)), then no file to
//! write refuses it, then the write itself, whose failure becomes
//! [`Error::WriteFailed`]. A refusal is an answer for the person, never a failure for the
//! caller (ADR-0046).

mod remove_api_keys;
mod remove_setting;
mod restore_setting;
mod write_setting;

pub use remove_api_keys::RemoveApiKeys;
pub use remove_setting::RemoveSetting;
pub use restore_setting::RestoreSetting;
pub use write_setting::WriteSetting;

use lib_core::configuration::ConfigKey;

use crate::device::{DeviceFile, DevicePorts};
use crate::error::{Error, Result};
use crate::protocols::DeviceAnswer;

/// Changes `key` in the file with `edit`, answering `value` once saved. A source above the file
/// that sets `key` refuses it instead, unless the key writes when shadowed.
fn change<T>(
    ports: &DevicePorts,
    key: ConfigKey,
    value: T,
    edit: impl FnOnce(&dyn DeviceFile) -> Result<()>,
) -> DeviceAnswer<T> {
    let file = ports.file();
    if !key.writes_when_shadowed()
        && let Some(source) = file.shadow(key)
    {
        return DeviceAnswer::Shadowed { source };
    }
    let Some(path) = file.target() else {
        return DeviceAnswer::Refused(Error::NoFile);
    };
    match edit(file) {
        Ok(()) => DeviceAnswer::Saved {
            value,
            file: path.display().to_string(),
        },
        Err(error) => DeviceAnswer::Refused(error),
    }
}

/// [`change`], noting the key's raw value before and after in the ports' log for the input's
/// History entry (#1244), even under an override.
fn recorded<T>(
    ports: &DevicePorts,
    key: ConfigKey,
    value: T,
    edit: impl FnOnce(&dyn DeviceFile) -> Result<()>,
) -> DeviceAnswer<T> {
    let Some(log) = ports.log() else {
        return change(ports, key, value, edit);
    };
    let before = ports.file().raw(key);
    let answer = change(ports, key, value, edit);
    if matches!(answer, DeviceAnswer::Saved { .. }) {
        log.record(key, before, ports.file().raw(key));
    }
    answer
}

/// How an answer ended, for the intent's log line.
fn outcome<T>(answer: &DeviceAnswer<T>) -> &'static str {
    match answer {
        DeviceAnswer::Saved { .. } => "saved",
        DeviceAnswer::Shadowed { .. } => "shadowed",
        DeviceAnswer::Refused(Error::NoFile) => "no file",
        DeviceAnswer::Refused(Error::WriteFailed) => "write failed",
    }
}
