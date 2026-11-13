//! What a module Dialog's Presenter takes and closes through: a [`DialogInput`], the Dialog
//! keymap command or typed key the Client resolved, and a [`DialogRouter`], the one Router every
//! module's Dialogs close through (#988).
//!
//! The Dialog keymap ([`dialog_keymap`]) lives here too (#1167), so a page Presenter that builds a
//! Dialog's keys on a rebind (Settings › Tracing) reaches it without `lib-shell`, which only the
//! bins may depend on and which re-exports it. The Client resolves a key through it and turns each
//! [`DialogCommand`] into its input. `back` isn't an input: the host decides it, closing an open
//! select first, otherwise cancelling the Dialog. A Dialog's Presenter reads the chords its hints
//! show from [`DialogKeys`], which the Client builds from that keymap and hands it at open
//! (ADR-0056).

use lib_toast::Notify;

use crate::keymap::{
    BACK, BACK_DEFAULT, Binding, GO_INTO, GO_UP, KeyBindingConfig, Keymap, LIST_NEXT,
    LIST_PREVIOUS, NEXT_FIELD, PREVIOUS_FIELD, SELECT, SELECT_NEXT, SELECT_PREVIOUS, SUBMIT,
};
use crate::view::{DialogHint, Hint};

/// One input a module Dialog's Presenter takes: a Dialog keymap command other than `back`, or a key
/// typed into the focused field. What it means is the Presenter's call, from its focused field and
/// whether its select is open: `Select` opens a closed select, picks from an open one, and
/// otherwise confirms only a valid form; `Submit` confirms a valid form from any field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogInput {
    Select,
    NextField,
    PreviousField,
    SelectNext,
    SelectPrevious,
    Submit,
    /// `←` and `→`, which no Dialog command binds: a field that steps along a row (the Tag form's
    /// swatches) reads them, and the others ignore them.
    Left,
    Right,
    /// A printable character, typed without a modifier.
    Char(char),
    Backspace,
}

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
    Binding::hidden(BACK, BACK_DEFAULT, DialogCommand::Back),
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

/// Where a module Dialog's Presenter asks to go: closing its Dialog, and Notices. A Dialog's Router
/// opens nothing, so this is the whole of it; what the page opens next is in the outcome. Generic
/// over the outcome `O`, so the Client implements it once for every module's Dialogs.
pub trait DialogRouter<O>: Notify {
    /// Closes the Dialog. `outcome` is what its confirm did, `None` when it was cancelled; the
    /// Client hands a confirmed one to the page that opened the Dialog.
    fn close(&mut self, outcome: Option<O>);
}

/// The chords the Dialog keymap binds its commands to now, after `[keybindings]`: what a Dialog's
/// Presenter builds its hints from, so its view model carries them as a page's does. The Client
/// builds it once from `lib-shell`'s Dialog keymap ([`Self::of`]) and a Router hands it to each
/// Dialog's Presenter at open; the Presenter never sees the keymap itself.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DialogKeys {
    /// Each command's chord, in [`Self::COMMANDS`] order. Empty for none: [`Self::default`], a
    /// Router that shows no hints.
    chords: Vec<(&'static str, String)>,
}

impl DialogKeys {
    /// The Dialog keymap's commands, the names this crate owns (`lib_viper::keymap`).
    pub const COMMANDS: [&'static str; 9] = [
        SELECT,
        BACK,
        NEXT_FIELD,
        PREVIOUS_FIELD,
        SELECT_NEXT,
        SELECT_PREVIOUS,
        SUBMIT,
        LIST_NEXT,
        LIST_PREVIOUS,
    ];

    /// The chords `keymap` binds the Dialog commands to: the Client passes `lib-shell`'s Dialog
    /// keymap, whatever its event type.
    pub fn of<E: Clone>(keymap: &Keymap<E>) -> Self {
        Self {
            chords: Self::COMMANDS
                .iter()
                .map(|command| {
                    let chord = keymap.hint_chord(command);
                    (*command, chord.to_string())
                })
                .collect(),
        }
    }

    /// The chord `command` is bound to, empty while unbound.
    pub fn chord(&self, command: &str) -> &str {
        self.chords
            .iter()
            .find(|(name, _)| *name == command)
            .map_or("", |(_, chord)| chord.as_str())
    }

    /// The hint for `command`: its chord and `action`, the Message saying what it does.
    pub fn hint(&self, command: &str, action: String) -> Hint {
        Hint::new(self.chord(command), action)
    }

    /// `hints` as `(chord, action)` pairs, each command's chord read from these keys: a hint of
    /// two commands reads `up/down`, an unbound one reads empty. It is [`legend`](crate::view::legend)
    /// over the chords a Dialog was handed, for a Dialog whose frame already lists its hints.
    pub fn legend(&self, hints: &[DialogHint]) -> Vec<Hint> {
        hints
            .iter()
            .map(|hint| {
                let chords: Vec<&str> = hint
                    .commands
                    .iter()
                    .map(|command| self.chord(command))
                    .collect();
                Hint::new(chords.join("/"), hint.action.clone())
            })
            .collect()
    }

    /// The list step's two chords as a hint shows them, `j/k` by default.
    pub fn list_step(&self) -> String {
        format!("{}/{}", self.chord(LIST_NEXT), self.chord(LIST_PREVIOUS))
    }
}

#[cfg(test)]
mod tests {
    use lib_config::KeyBindingConfig;
    use lib_toast::Notice;

    use super::*;
    use crate::keymap::Binding;

    const DIALOG_BINDINGS: &[Binding<()>] = &[
        Binding::hidden(BACK, BACK_DEFAULT, ()),
        Binding::hidden(SELECT, "enter", ()),
        Binding::hidden(LIST_NEXT, "j", ()),
        Binding::hidden(LIST_PREVIOUS, "k", ()),
    ];

    #[test]
    fn dialog_keys_read_each_commands_chord_from_the_keymap_after_any_override() {
        let mut config = KeyBindingConfig::default();
        config
            .bindings
            .insert(BACK.to_string(), "ctrl+g".to_string());
        let keys = DialogKeys::of(&Keymap::new(DIALOG_BINDINGS, &config));

        assert_eq!(keys.chord(BACK), "ctrl+g");
        assert_eq!(keys.chord(SELECT), "enter");
        assert_eq!(
            keys.chord(SUBMIT),
            "",
            "a command the keymap doesn't bind reads empty"
        );
        assert_eq!(
            keys.chord("units.set_base"),
            "",
            "nor does a page's command"
        );
        assert_eq!(keys.list_step(), "j/k");
        assert_eq!(
            keys.hint(SELECT, "confirm".to_string()),
            Hint::new("enter", "confirm")
        );
    }

    #[test]
    fn default_dialog_keys_bind_nothing() {
        let keys = DialogKeys::default();

        assert!(
            DialogKeys::COMMANDS
                .iter()
                .all(|command| keys.chord(command).is_empty())
        );
    }

    /// A Router recording what it was asked, as a Client's would act on it.
    #[derive(Default)]
    struct Recording {
        closed: Vec<Option<&'static str>>,
        notices: Vec<Notice>,
    }

    impl DialogRouter<&'static str> for Recording {
        fn close(&mut self, outcome: Option<&'static str>) {
            self.closed.push(outcome);
        }
    }

    impl Notify for Recording {
        fn notify(&mut self, notice: Notice) {
            self.notices.push(notice);
        }
    }

    /// A Presenter that confirms on `Submit` and cancels on anything else, through any module's
    /// Router.
    fn presenter(input: DialogInput, router: &mut dyn DialogRouter<&'static str>) {
        if input == DialogInput::Submit {
            router.notify(Notice::Status("saved".to_string()));
            router.close(Some("saved"));
        } else {
            router.close(None);
        }
    }

    #[test]
    fn a_presenter_closes_with_its_outcome_and_notifies_through_one_router() {
        let mut router = Recording::default();

        presenter(DialogInput::Submit, &mut router);
        presenter(DialogInput::Char('x'), &mut router);

        assert_eq!(router.closed, [Some("saved"), None]);
        assert_eq!(router.notices, [Notice::Status("saved".to_string())]);
    }

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
        assert_eq!(crate::keymap::list_step_chords(&keymap), "ctrl+j/k");
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
