//! The Dialog keymap: the keys every Dialog shares, so `Enter`, `Esc`, `Tab` and select stepping
//! behave the same in each and follow `[keybindings]` (#820, Q1, Q14, Q15). The Client resolves a
//! key through it while a Dialog is open and hands the open Dialog the [`DialogCommand`]; a key it
//! doesn't bind is the Dialog's own, typing into a field.
//!
//! What a command does is the Dialog's call, not the Client's: [`DialogCommand::Select`] opens a
//! closed select, picks the highlighted Choice on an open one, and otherwise confirms only a valid
//! form; [`DialogCommand::Submit`] confirms a valid form from anywhere, a multi-line box or a
//! select included. Only [`DialogCommand::Back`] the host decides itself: it closes an open select
//! first, otherwise cancels the Dialog and closes it.

use lib_config::KeyBindingConfig;
use lib_viper::keymap::{
    BACK, Binding, Keymap, NEXT_FIELD, PREVIOUS_FIELD, SELECT, SELECT_NEXT, SELECT_PREVIOUS, SUBMIT,
};

/// A key the Dialog keymap binds, as the open Dialog receives it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogCommand {
    Select,
    Back,
    NextField,
    PreviousField,
    SelectNext,
    SelectPrevious,
    Submit,
}

/// The Dialog keymap's commands with their default keys.
pub const DIALOG_BINDINGS: &[Binding<DialogCommand>] = &[
    Binding {
        command: SELECT,
        chord: "enter",
        event: DialogCommand::Select,
    },
    Binding {
        command: BACK,
        chord: "esc",
        event: DialogCommand::Back,
    },
    Binding {
        command: NEXT_FIELD,
        chord: "tab",
        event: DialogCommand::NextField,
    },
    Binding {
        command: PREVIOUS_FIELD,
        chord: "shift+tab",
        event: DialogCommand::PreviousField,
    },
    Binding {
        command: SELECT_NEXT,
        chord: "down",
        event: DialogCommand::SelectNext,
    },
    Binding {
        command: SELECT_PREVIOUS,
        chord: "up",
        event: DialogCommand::SelectPrevious,
    },
    Binding {
        command: SUBMIT,
        chord: "ctrl+enter",
        event: DialogCommand::Submit,
    },
];

/// The Dialog keymap under `config`'s overrides.
pub fn dialog_keymap(config: &KeyBindingConfig) -> Keymap<DialogCommand> {
    Keymap::new(DIALOG_BINDINGS, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn overriding(command: &str, chord: &str) -> KeyBindingConfig {
        let mut config = KeyBindingConfig::default();
        config
            .bindings
            .insert(command.to_string(), chord.to_string());
        config
    }

    #[test]
    fn each_default_key_reaches_its_command() {
        let keymap = dialog_keymap(&KeyBindingConfig::default());

        for (chord, command) in [
            ("enter", DialogCommand::Select),
            ("esc", DialogCommand::Back),
            ("tab", DialogCommand::NextField),
            ("shift+tab", DialogCommand::PreviousField),
            ("down", DialogCommand::SelectNext),
            ("up", DialogCommand::SelectPrevious),
            ("ctrl+enter", DialogCommand::Submit),
        ] {
            assert_eq!(keymap.event(chord), Some(command), "{chord}");
        }
        assert_eq!(keymap.event("j"), None, "typing is the Dialog's own");
    }

    #[test]
    fn an_override_moves_only_its_command() {
        let keymap = dialog_keymap(&overriding(NEXT_FIELD, "ctrl+n"));

        assert_eq!(keymap.event("ctrl+n"), Some(DialogCommand::NextField));
        assert_eq!(keymap.event("tab"), None, "the default no longer binds");
        assert_eq!(
            keymap.event("shift+tab"),
            Some(DialogCommand::PreviousField)
        );
    }

    #[test]
    fn back_and_select_follow_the_overrides_views_already_read() {
        let keymap = dialog_keymap(&overriding(BACK, "ctrl+h"));

        assert_eq!(keymap.event("ctrl+h"), Some(DialogCommand::Back));
        assert_eq!(keymap.event("esc"), None);
        assert_eq!(keymap.chord(SELECT), Some("enter"));
    }
}
