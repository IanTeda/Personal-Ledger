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
//!
//! [`DialogCommand::ListNext`] and [`DialogCommand::ListPrevious`] are bound to letters, so they
//! are a step only in a Dialog stepping a list right then (the Pay Dialog's Match list, the
//! Budgets lists); every other Dialog and field takes the key as typing
//! ([`DialogCommand::is_list_step`]).

use lib_config::KeyBindingConfig;
use lib_viper::dialog::DialogInput;
use lib_viper::keymap::{
    BACK, Binding, GO_INTO, GO_UP, Keymap, LIST_NEXT, LIST_PREVIOUS, NEXT_FIELD, PREVIOUS_FIELD,
    SELECT, SELECT_NEXT, SELECT_PREVIOUS, SUBMIT,
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
    /// A list Dialog's row step, typing in any other (see the module docs).
    ListNext,
    ListPrevious,
    /// The folder picker's step into the folder under the cursor, typing everywhere else.
    GoInto,
    /// The folder picker's step up to the parent folder, typing everywhere else.
    GoUp,
}

/// The Dialog keymap's commands with their default keys.
pub const DIALOG_BINDINGS: &[Binding<DialogCommand>] = &[
    Binding::hidden(SELECT, "enter", DialogCommand::Select),
    Binding::hidden(BACK, "esc", DialogCommand::Back),
    Binding::hidden(NEXT_FIELD, "tab", DialogCommand::NextField),
    Binding::hidden(PREVIOUS_FIELD, "shift+tab", DialogCommand::PreviousField),
    Binding::hidden(SELECT_NEXT, "down", DialogCommand::SelectNext),
    Binding::hidden(SELECT_PREVIOUS, "up", DialogCommand::SelectPrevious),
    Binding::hidden(SUBMIT, "ctrl+enter", DialogCommand::Submit),
    // The list-stepping chords of a terminal: a second chord for the same two commands, so every
    // Dialog list takes them (the link picker did before `DialogInput` dropped Ctrl keys, #990).
    // They sit last so a rebind that moves another command onto `ctrl+n` wins it.
    Binding::hidden(SELECT_NEXT, "ctrl+n", DialogCommand::SelectNext),
    Binding::hidden(SELECT_PREVIOUS, "ctrl+p", DialogCommand::SelectPrevious),
    Binding::hidden(LIST_NEXT, "j", DialogCommand::ListNext),
    Binding::hidden(LIST_PREVIOUS, "k", DialogCommand::ListPrevious),
    Binding::hidden(GO_INTO, "l", DialogCommand::GoInto),
    Binding::hidden(GO_UP, "h", DialogCommand::GoUp),
    Binding::hidden(GO_UP, "backspace", DialogCommand::GoUp),
];

impl DialogCommand {
    /// The command as a module Dialog's Presenter takes it (`lib_viper::dialog`), `None` for
    /// `back`, which the host decides. A list step is a select step: the host sends one only to a
    /// Dialog stepping a list, and types the key into any other.
    pub fn input(self) -> Option<DialogInput> {
        match self {
            Self::Select => Some(DialogInput::Select),
            Self::NextField => Some(DialogInput::NextField),
            Self::PreviousField => Some(DialogInput::PreviousField),
            Self::SelectNext | Self::ListNext => Some(DialogInput::SelectNext),
            Self::SelectPrevious | Self::ListPrevious => Some(DialogInput::SelectPrevious),
            Self::Submit => Some(DialogInput::Submit),
            Self::Back | Self::GoInto | Self::GoUp => None,
        }
    }

    /// A list Dialog's row step, which a Dialog or field not stepping a list takes as typing.
    pub fn is_list_step(self) -> bool {
        matches!(self, Self::ListNext | Self::ListPrevious)
    }

    /// The folder picker's own step, which every other host takes as typing.
    pub fn is_folder_step(self) -> bool {
        matches!(self, Self::GoInto | Self::GoUp)
    }
}

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
            ("ctrl+n", DialogCommand::SelectNext),
            ("ctrl+p", DialogCommand::SelectPrevious),
            ("j", DialogCommand::ListNext),
            ("k", DialogCommand::ListPrevious),
            ("l", DialogCommand::GoInto),
            ("h", DialogCommand::GoUp),
            ("backspace", DialogCommand::GoUp),
        ] {
            assert_eq!(keymap.event(chord), Some(command), "{chord}");
        }
        assert_eq!(keymap.event("x"), None, "typing is the Dialog's own");
    }

    #[test]
    fn a_list_step_is_a_select_step_and_rebinds_on_its_own() {
        assert_eq!(
            DialogCommand::ListNext.input(),
            Some(DialogInput::SelectNext)
        );
        assert!(DialogCommand::ListPrevious.is_list_step());
        assert!(!DialogCommand::SelectNext.is_list_step());

        let keymap = dialog_keymap(&overriding(LIST_NEXT, "ctrl+j"));

        assert_eq!(keymap.event("ctrl+j"), Some(DialogCommand::ListNext));
        assert_eq!(keymap.event("j"), None, "the default no longer binds");
        assert_eq!(lib_viper::keymap::list_step_chords(&keymap), "ctrl+j/k");
    }

    #[test]
    fn a_folder_step_rebinds_on_its_own_and_types_elsewhere() {
        assert!(DialogCommand::GoInto.is_folder_step());
        assert!(!DialogCommand::ListNext.is_folder_step());
        assert_eq!(DialogCommand::GoUp.input(), None);

        let keymap = dialog_keymap(&overriding(GO_UP, "left"));

        assert_eq!(keymap.event("left"), Some(DialogCommand::GoUp));
        assert_eq!(keymap.event("h"), None, "the default no longer binds");
        assert_eq!(keymap.event("backspace"), None);
        assert_eq!(keymap.event("l"), Some(DialogCommand::GoInto));
    }

    #[test]
    fn every_command_but_back_is_a_presenters_input() {
        assert_eq!(DialogCommand::Back.input(), None, "back is the host's");
        assert_eq!(
            DialogCommand::PreviousField.input(),
            Some(DialogInput::PreviousField)
        );
        assert_eq!(DialogCommand::Submit.input(), Some(DialogInput::Submit));
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
    fn ctrl_n_and_ctrl_p_step_a_list_beside_the_arrows() {
        let keymap = dialog_keymap(&KeyBindingConfig::default());

        assert_eq!(keymap.event("ctrl+n"), Some(DialogCommand::SelectNext));
        assert_eq!(keymap.event("ctrl+p"), Some(DialogCommand::SelectPrevious));
        assert_eq!(keymap.event("down"), Some(DialogCommand::SelectNext));
        assert_eq!(keymap.event("up"), Some(DialogCommand::SelectPrevious));
    }

    #[test]
    fn a_rebound_next_field_keeps_ctrl_n() {
        // `next_field` is bound earlier, so on `ctrl+n` it wins and the list step gives way.
        let keymap = dialog_keymap(&overriding(NEXT_FIELD, "ctrl+n"));

        assert_eq!(keymap.event("ctrl+n"), Some(DialogCommand::NextField));
        assert_eq!(keymap.event("ctrl+p"), Some(DialogCommand::SelectPrevious));
    }

    #[test]
    fn back_and_select_follow_the_overrides_views_already_read() {
        let keymap = dialog_keymap(&overriding(BACK, "ctrl+h"));

        assert_eq!(keymap.event("ctrl+h"), Some(DialogCommand::Back));
        assert_eq!(keymap.event("esc"), None);
        assert_eq!(keymap.chord(SELECT), Some("enter"));
    }
}
