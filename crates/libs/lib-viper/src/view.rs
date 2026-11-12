//! The pieces a Presenter builds a form or Dialog's view model from, as plain data a View only
//! draws. A text field's caret isn't here: it is the View's, as widget mechanics of its toolkit.
//! A select's open list and highlight are, as [`SelectList`], for a Dialog whose Presenter takes
//! [`DialogInput`]s and so must know whether its select is open.

use crate::dialog::DialogInput;
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

/// A select's list as a Presenter whose Dialog takes [`DialogInput`]s holds it: whether it is
/// open and, while it is, the option highlighted, with the moves a Dialog input makes on them. The
/// Presenter hands it to the View beside the select's [`SelectView`]; the View only draws the list
/// (its scroll window too) and hit-tests a click on a row.
///
/// A Dialog still on the bin's key path (#991) keeps its list in the bin's `SelectState` instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SelectList {
    /// The highlighted option while open, `None` while closed.
    highlight: Option<usize>,
}

/// What a Dialog input did on a focused select.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectMove<T> {
    /// It picked this option: stepped a closed select's value, or committed an open list's
    /// highlight.
    Chose(T),
    /// It opened the list or moved its highlight.
    Moved,
    /// Not a select's input; the Presenter decides what it means.
    Ignored,
}

impl SelectList {
    pub fn is_open(self) -> bool {
        self.highlight.is_some()
    }

    /// The highlighted option while the list is open.
    pub fn highlight(self) -> Option<usize> {
        self.highlight
    }

    /// Opens the list on `chosen`, or on the first option when none of `options` is chosen. An
    /// empty select has no list to open.
    pub fn open<T: PartialEq>(&mut self, options: &[Choice<T>], chosen: Option<&T>) {
        if !options.is_empty() {
            self.highlight = Some(position(options, chosen).unwrap_or(0));
        }
    }

    /// Opens a closed list, or closes an open one: a click on the select's field.
    pub fn toggle<T: PartialEq>(&mut self, options: &[Choice<T>], chosen: Option<&T>) {
        if !self.close() {
            self.open(options, chosen);
        }
    }

    /// Closes the list without choosing. Returns whether it was open.
    pub fn close(&mut self) -> bool {
        self.highlight.take().is_some()
    }

    /// Closes the list, returning its highlighted option when it was open: leaving the select
    /// commits the highlight, as a click on it would.
    pub fn commit<T: Clone>(&mut self, options: &[Choice<T>]) -> Option<T> {
        self.highlight
            .take()
            .and_then(|index| options.get(index))
            .map(|choice| choice.value.clone())
    }

    /// `input` on the focused select over `options`, of which `chosen` is chosen. `Select` (or
    /// `Space`) opens a closed list and commits an open one's highlight; `SelectNext` and
    /// `SelectPrevious` step a closed select's value or move an open list's highlight, wrapping at
    /// either end. A closed select with nothing chosen steps onto the first or last option.
    pub fn input<T: Clone + PartialEq>(
        &mut self,
        input: DialogInput,
        options: &[Choice<T>],
        chosen: Option<&T>,
    ) -> SelectMove<T> {
        if options.is_empty() {
            return SelectMove::Ignored;
        }
        let delta = match input {
            DialogInput::SelectNext => 1,
            DialogInput::SelectPrevious => -1,
            DialogInput::Select | DialogInput::Char(' ') => {
                return match self.commit(options) {
                    Some(value) => SelectMove::Chose(value),
                    None => {
                        self.open(options, chosen);
                        SelectMove::Moved
                    }
                };
            }
            _ => return SelectMove::Ignored,
        };
        let len = options.len();
        match self.highlight {
            Some(index) => {
                self.highlight = Some(wrap(index.min(len - 1), delta, len));
                SelectMove::Moved
            }
            None => {
                let next = match position(options, chosen) {
                    Some(index) => wrap(index, delta, len),
                    None if delta < 0 => len - 1,
                    None => 0,
                };
                SelectMove::Chose(options[next].value.clone())
            }
        }
    }
}

fn position<T: PartialEq>(options: &[Choice<T>], chosen: Option<&T>) -> Option<usize> {
    let chosen = chosen?;
    options.iter().position(|choice| &choice.value == chosen)
}

/// `index` moved by `delta`, wrapping within `len`.
fn wrap(index: usize, delta: isize, len: usize) -> usize {
    let len = isize::try_from(len).unwrap_or(isize::MAX);
    let index = isize::try_from(index).unwrap_or(0);
    usize::try_from((index + delta).rem_euclid(len)).unwrap_or(0)
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
        legend(&self.hints, keymap)
    }
}

/// `hints` as `(chord, action)` pairs, each chord read from `keymap` so a `[keybindings]` override
/// shows. A hint of two commands reads `up/down`; an unbound command reads empty. A View with no
/// [`DialogFrame`] (the Import step) reads its hints through this too.
pub fn legend<E: Clone>(hints: &[DialogHint], keymap: &Keymap<E>) -> Vec<Hint> {
    hints
        .iter()
        .map(|hint| {
            let chords: Vec<&str> = hint
                .commands
                .iter()
                .map(|command| keymap.hint_chord(command))
                .collect();
            Hint {
                chord: chords.join("/"),
                action: hint.action.clone(),
            }
        })
        .collect()
}

/// A count as a Message's plural argument, which the generated accessors take as `i64`.
pub fn count(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
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

    fn options(n: usize) -> Vec<Choice<usize>> {
        (0..n)
            .map(|value| Choice {
                value,
                label: format!("option {value}"),
            })
            .collect()
    }

    #[test]
    fn a_closed_select_steps_its_value_and_wraps() {
        let options = options(3);
        let mut list = SelectList::default();

        assert_eq!(
            list.input(DialogInput::SelectNext, &options, Some(&1)),
            SelectMove::Chose(2)
        );
        assert_eq!(
            list.input(DialogInput::SelectNext, &options, Some(&2)),
            SelectMove::Chose(0)
        );
        assert_eq!(
            list.input(DialogInput::SelectPrevious, &options, Some(&0)),
            SelectMove::Chose(2)
        );
        assert!(!list.is_open());
    }

    #[test]
    fn a_closed_select_with_nothing_chosen_steps_onto_either_end() {
        let options = options(3);
        let mut list = SelectList::default();

        assert_eq!(
            list.input(DialogInput::SelectNext, &options, None),
            SelectMove::Chose(0)
        );
        assert_eq!(
            list.input(DialogInput::SelectPrevious, &options, Some(&7)),
            SelectMove::Chose(2),
            "a value not offered counts as nothing chosen"
        );
    }

    #[test]
    fn select_opens_on_the_chosen_option_the_arrows_move_and_select_picks() {
        let options = options(3);
        let mut list = SelectList::default();

        assert_eq!(
            list.input(DialogInput::Select, &options, Some(&1)),
            SelectMove::Moved
        );
        assert_eq!(list.highlight(), Some(1));
        list.input(DialogInput::SelectNext, &options, Some(&1));
        list.input(DialogInput::SelectNext, &options, Some(&1));
        assert_eq!(list.highlight(), Some(0), "the highlight wraps");
        assert_eq!(
            list.input(DialogInput::SelectPrevious, &options, Some(&1)),
            SelectMove::Moved
        );

        assert_eq!(
            list.input(DialogInput::Char(' '), &options, Some(&1)),
            SelectMove::Chose(2),
            "space picks as select does"
        );
        assert!(!list.is_open());
    }

    #[test]
    fn typing_is_no_selects_and_an_empty_select_never_opens() {
        let mut list = SelectList::default();
        assert_eq!(
            list.input(DialogInput::Char('x'), &options(3), None),
            SelectMove::Ignored
        );
        assert_eq!(
            list.input(DialogInput::Select, &options(0), None),
            SelectMove::Ignored
        );
        list.open(&options(0), None);
        assert!(!list.is_open());
    }

    #[test]
    fn close_and_toggle_report_and_flip_the_list_and_commit_takes_the_highlight() {
        let options = options(3);
        let mut list = SelectList::default();
        assert!(!list.close(), "nothing was open");

        list.toggle(&options, Some(&2));
        assert_eq!(list.highlight(), Some(2));
        list.toggle(&options, Some(&2));
        assert!(!list.is_open());

        list.open(&options, None);
        assert_eq!(list.highlight(), Some(0));
        assert!(list.close());
        assert_eq!(list.commit(&options), None, "a closed list commits nothing");
        list.open(&options, Some(&1));
        assert_eq!(list.commit(&options), Some(1));
        assert!(!list.is_open());
    }

    #[test]
    fn a_count_too_big_for_a_message_saturates() {
        assert_eq!(count(3), 3);
        assert_eq!(count(usize::MAX), i64::MAX);
    }
}
