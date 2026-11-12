//! What the Settings pages share beside the device settings form (`form.rs`): the stock note
//! under a field whose key a source above the file sets, and a command's chords for a hint.

use lib_core::configuration::ConfigKey;
use lib_viper::keymap::Keymap;

use crate::msg;
use crate::protocols::DeviceSnapshot;

/// The note under a control whose key a source above the file sets.
pub(super) fn shadowed_note(snapshot: &DeviceSnapshot, key: ConfigKey) -> Option<String> {
    snapshot
        .get(key)
        .shadow
        .as_deref()
        .map(msg::settings_device_shadowed_note)
}

/// The chord `command` is bound to, for a hint.
pub(super) fn chord<E: Clone>(keymap: &Keymap<E>, command: &str) -> String {
    keymap.chord(command).unwrap_or_default().to_string()
}

/// Commands' chords as one hint shows them, `j/k`.
pub(super) fn chords<E: Clone>(keymap: &Keymap<E>, commands: &[&str]) -> String {
    commands
        .iter()
        .map(|command| chord(keymap, command))
        .collect::<Vec<_>>()
        .join("/")
}
