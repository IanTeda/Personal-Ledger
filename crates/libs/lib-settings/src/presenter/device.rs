//! What the This device pages share: the keys they take ([`PageKey`]), a control as drawn
//! ([`ControlView`]), and how a [`DeviceConfiguration`] write's answer reaches the person, on the
//! status line, before the page takes the saved value.

use lib_list_cursor::Step;
use lib_toast::{Notice, Notify};
use lib_viper::keymap::Keymap;
use lib_viper::view::SelectView;

use crate::msg;
use crate::protocols::{DeviceAnswer, DeviceConfiguration, DeviceKey, DeviceRefusal};

/// A key on a This device page, from [`crate::keymap::device_page_keymap`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageKey {
    /// The control below or above.
    Move(Step),
    /// The focused control's choice after or before.
    Change(Step),
    /// `enter`: ticks the checkbox, or chooses the Colour Theme the grid's focus is on.
    Select,
    /// `space`: ticks the checkbox.
    Toggle,
}

/// One control as drawn: its choices and the one chosen, the line under it, and, when a source
/// above the configuration file sets its key, the note saying so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlView<T> {
    pub select: SelectView<T>,
    pub hint: Option<String>,
    pub shadowed: Option<String>,
}

/// The note under a control whose key a source above the file sets.
pub(super) fn shadowed_note(device: &dyn DeviceConfiguration, key: DeviceKey) -> Option<String> {
    device
        .shadow(key)
        .map(|source| msg::settings_device_shadowed_note(&source))
}

/// Says on the status line how the write of `key` went, and hands back the value saved, `None`
/// when it wasn't, so the page keeps what it had.
pub(super) fn settle<T>(
    answer: DeviceAnswer<T>,
    key: DeviceKey,
    router: &mut dyn Notify,
) -> Option<T> {
    let name = key.name();
    let (status, saved) = match answer {
        DeviceAnswer::Saved { value, file } => {
            (msg::settings_device_saved(name, &file), Some(value))
        }
        DeviceAnswer::Shadowed { source } => (msg::settings_device_shadowed(name, &source), None),
        DeviceAnswer::Refused(DeviceRefusal::NoFile) => (msg::settings_device_no_file(name), None),
        DeviceAnswer::Refused(DeviceRefusal::WriteFailed) => {
            (msg::settings_device_write_failed(name), None)
        }
    };
    router.notify(Notice::Status(status));
    saved
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

/// The choice `step` from `current` among `choices`, stopping at either end rather than wrapping.
/// A `current` that isn't among them steps from `from`.
pub(super) fn step_choice<T: Clone + PartialEq>(
    choices: &[T],
    current: &T,
    from: &T,
    step: Step,
) -> Option<T> {
    let position = choices
        .iter()
        .position(|choice| choice == current)
        .or_else(|| choices.iter().position(|choice| choice == from))?;
    let next = lib_list_cursor::step(position, choices.len(), step);
    choices.get(next).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_step_stops_at_either_end() {
        let choices = [1, 2, 3];
        assert_eq!(step_choice(&choices, &2, &1, Step::Next), Some(3));
        assert_eq!(step_choice(&choices, &3, &1, Step::Next), Some(3));
        assert_eq!(step_choice(&choices, &1, &1, Step::Prev), Some(1));
    }

    #[test]
    fn a_value_not_among_the_choices_steps_from_the_fallback() {
        assert_eq!(step_choice(&[1, 2, 3], &9, &1, Step::Next), Some(2));
    }
}
