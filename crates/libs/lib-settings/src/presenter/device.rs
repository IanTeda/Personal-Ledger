//! What the Settings pages share beside the device settings form (`form.rs`): the stock note
//! under a field whose key a source above the file sets, and a command's chords for a hint.

use lib_viper::keymap::Keymap;

use crate::msg;

/// The note under a control whose key `source`, a source above the file, sets; `None` for none.
pub(super) fn shadowed_note(source: Option<&str>) -> Option<String> {
    source.map(msg::settings_device_shadowed_note)
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
