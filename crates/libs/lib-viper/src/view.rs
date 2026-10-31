//! The pieces a Presenter builds a form or Dialog's view model from, as plain data a View only
//! draws. A text field's caret and a select's open list aren't here: they are the View's, as
//! widget mechanics of its toolkit.

use crate::keymap::Keymap;

/// One option of a select or segmented control: the value it sets and the text it shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice<T> {
    pub value: T,
    pub label: String,
}

/// A text field as drawn: its label, what it holds, its placeholder and whether it has focus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextFieldView {
    pub label: String,
    pub text: String,
    pub placeholder: String,
    pub focused: bool,
}

/// A select or segmented control as drawn: its label, options, the value chosen and whether it
/// has focus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectView<T> {
    pub label: String,
    pub options: Vec<Choice<T>>,
    pub chosen: Option<T>,
    pub focused: bool,
}

/// A Dialog's buttons and title: the submit's label and whether it is enabled, whether it
/// destroys something (drawn in the danger colour), and the other keys it takes, in drawn order.
/// The Desktop may leave the hints undrawn, since its Dialogs have buttons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogFrame {
    pub title: String,
    pub submit: String,
    pub can_confirm: bool,
    pub destructive: bool,
    pub hints: Vec<DialogHint>,
}

impl DialogFrame {
    /// The hints as `(chord, action)` pairs, each chord read from `keymap`, the Dialog keymap the
    /// Client looks keys up in, so a `[keybindings]` override shows. A hint of two commands reads
    /// `up/down`; an unbound command reads empty.
    pub fn legend<E: Clone>(&self, keymap: &Keymap<E>) -> Vec<Hint> {
        self.hints
            .iter()
            .map(|hint| {
                let chords: Vec<&str> = hint
                    .commands
                    .iter()
                    .map(|command| keymap.chord(command).unwrap_or_default())
                    .collect();
                Hint {
                    chord: chords.join("/"),
                    action: hint.action.clone(),
                }
            })
            .collect()
    }
}

/// One key a View or Dialog takes, as the status line and the `?` cheat sheet show it: the chord
/// its command is bound to now, after `[keybindings]`, and what it does. Every Presenter's legend
/// is a list of these, so a Client draws them without an adapter per module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hint {
    pub chord: String,
    pub action: String,
}

impl Hint {
    pub fn new(chord: impl Into<String>, action: impl Into<String>) -> Self {
        Self {
            chord: chord.into(),
            action: action.into(),
        }
    }
}

/// One key a Dialog takes besides its submit: the Dialog keymap commands that press it and what it
/// does. The Presenter names the commands and the Client reads their chords
/// ([`DialogFrame::legend`]), since the Dialog keymap is `lib-shell`'s, which only the bins may
/// depend on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DialogHint {
    pub commands: &'static [&'static str],
    pub action: String,
}

#[cfg(test)]
mod tests {
    use lib_config::KeyBindingConfig;

    use super::*;
    use crate::keymap::{BACK, Binding, SELECT_NEXT, SELECT_PREVIOUS};

    const BINDINGS: &[Binding<()>] = &[
        Binding::hidden(BACK, "esc", ()),
        Binding::hidden(SELECT_NEXT, "down", ()),
        Binding::hidden(SELECT_PREVIOUS, "up", ()),
    ];

    fn frame() -> DialogFrame {
        DialogFrame {
            title: String::new(),
            submit: String::new(),
            can_confirm: true,
            destructive: false,
            hints: vec![
                DialogHint {
                    commands: &[SELECT_PREVIOUS, SELECT_NEXT],
                    action: "choose".to_string(),
                },
                DialogHint {
                    commands: &[BACK],
                    action: "cancel".to_string(),
                },
            ],
        }
    }

    fn pairs(legend: &[(&str, &str)]) -> Vec<Hint> {
        legend
            .iter()
            .map(|(chord, action)| Hint {
                chord: chord.to_string(),
                action: action.to_string(),
            })
            .collect()
    }

    #[test]
    fn the_legend_reads_each_chord_from_the_keymap_in_drawn_order() {
        let keymap = Keymap::new(BINDINGS, &KeyBindingConfig::default());

        assert_eq!(
            frame().legend(&keymap),
            pairs(&[("up/down", "choose"), ("esc", "cancel")])
        );
    }

    #[test]
    fn the_legend_follows_an_override() {
        let mut config = KeyBindingConfig::default();
        config
            .bindings
            .insert(BACK.to_string(), "ctrl+h".to_string());
        let keymap = Keymap::new(BINDINGS, &config);

        assert_eq!(
            frame().legend(&keymap)[1],
            pairs(&[("ctrl+h", "cancel")])[0]
        );
    }
}
