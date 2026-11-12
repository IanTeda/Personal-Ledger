//! What a module Dialog's Presenter takes and closes through: a [`DialogInput`], the Dialog
//! keymap command or typed key the Client resolved, and a [`DialogRouter`], the one Router every
//! module's Dialogs close through (#988).
//!
//! The Dialog keymap itself is `lib-shell`'s (`lib_shell::dialog`), which only the bins may
//! depend on, so a feature crate names the commands here and the Client turns each into its input.
//! `back` isn't an input: the host decides it, closing an open select first, otherwise cancelling
//! the Dialog. For the same reason a Dialog's Presenter reads the chords its hints show from
//! [`DialogKeys`], which the Client builds from that keymap and hands it at open (ADR-0056).

use lib_toast::Notify;

use crate::keymap::{
    BACK, Keymap, LIST_NEXT, LIST_PREVIOUS, NEXT_FIELD, PREVIOUS_FIELD, SELECT, SELECT_NEXT,
    SELECT_PREVIOUS, SUBMIT,
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
        Binding::hidden(BACK, "esc", ()),
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
}
