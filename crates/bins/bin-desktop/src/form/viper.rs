//! The pieces a View of a VIPER module's form shares: turning a typed key into a text field's new
//! text, and keeping a select's open list and highlight (widget state, the View's) in line with
//! the option the Presenter chose (`lib_viper::view::SelectView`).

use lib_viper::view::SelectView;

use crate::chrome::dialog_host::{DialogKey, DialogOutcome};
use crate::form::select::SelectState;

/// What `Enter` does on a form: confirm while the Presenter would, else nothing.
pub(crate) fn enter(can_confirm: bool) -> DialogOutcome {
    if can_confirm {
        DialogOutcome::Confirm
    } else {
        DialogOutcome::Handled
    }
}

/// `text` with a typed key applied: a character appended or, on `Backspace`, the last removed.
/// `None` for any other key.
pub(crate) fn typed(text: &str, key: DialogKey) -> Option<String> {
    let mut text = text.to_string();
    match key {
        DialogKey::Char(ch) => text.push(ch),
        DialogKey::Backspace => {
            text.pop();
        }
        _ => return None,
    }
    Some(text)
}

/// A select's option labels, as `SelectState` keys them.
pub(crate) fn labels<T>(view: &SelectView<T>) -> Vec<String> {
    view.options
        .iter()
        .map(|choice| choice.label.clone())
        .collect()
}

/// The label of the option `view` has chosen.
pub(crate) fn chosen_label<T: PartialEq>(view: &SelectView<T>) -> Option<String> {
    view.options
        .iter()
        .find(|choice| Some(&choice.value) == view.chosen.as_ref())
        .map(|choice| choice.label.clone())
}

/// `state` showing `view`'s chosen option, closed if the Presenter chose another under it.
pub(crate) fn synced<T: PartialEq>(state: &SelectState, view: &SelectView<T>) -> SelectState {
    let chosen = chosen_label(view);
    if state.value() == chosen.as_deref() {
        state.clone()
    } else {
        SelectState::new(chosen)
    }
}

/// The value of the option labelled `label`.
fn value_of<T: Copy>(view: &SelectView<T>, label: Option<&str>) -> Option<T> {
    let label = label?;
    view.options
        .iter()
        .find(|choice| choice.label == label)
        .map(|choice| choice.value)
}

/// A select's keys: `Up`/`Down` step a closed list's value or move an open list's highlight,
/// `Enter`/`Space` open a closed list or commit an open one. Returns the value to send when the
/// chosen option changed, and whether the key was a select key at all.
pub(crate) fn select_key<T: Copy + PartialEq>(
    state: &mut SelectState,
    view: &SelectView<T>,
    key: DialogKey,
) -> (bool, Option<T>) {
    *state = synced(state, view);
    let options = labels(view);
    let before = state.value().map(str::to_string);
    match key {
        DialogKey::Up | DialogKey::Down => {
            let delta = if key == DialogKey::Up { -1 } else { 1 };
            if state.is_open() {
                state.move_highlight(&options, delta);
            } else {
                state.step(&options, delta);
            }
        }
        DialogKey::Enter | DialogKey::Char(' ') => {
            if state.is_open() {
                state.commit(&options);
            } else {
                state.open(&options);
            }
        }
        _ => return (false, None),
    }
    let changed = state.value() != before.as_deref();
    (
        true,
        changed.then(|| value_of(view, state.value())).flatten(),
    )
}

/// A click on a select's field: opens its list, or closes it when open.
pub(crate) fn toggle_select<T: PartialEq>(state: &mut SelectState, view: &SelectView<T>) {
    *state = synced(state, view);
    if state.is_open() {
        state.cancel();
    } else {
        state.open(&labels(view));
    }
}

/// A click on row `index` of a select's open list: the value to send.
pub(crate) fn choose_option<T: Copy + PartialEq>(
    state: &mut SelectState,
    view: &SelectView<T>,
    index: usize,
) -> Option<T> {
    *state = synced(state, view);
    state.choose(&labels(view), index);
    value_of(view, state.value())
}
