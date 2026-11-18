//! The pieces a View of a VIPER module's form shares: a select's labels and chosen label, and the
//! select as the shared dropdown draws it, its list and highlight the Presenter's
//! (`lib_viper::view::SelectView`).

use lib_viper::view::{SelectList, SelectView};

use crate::form::select::SelectState;

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

/// `view` with the list its Presenter holds, as the shared dropdown draws it. `scroll` is the open
/// list's window, which the View keeps between draws so the window follows the highlight.
pub(crate) fn drawn<T: PartialEq>(
    view: &SelectView<T>,
    list: SelectList,
    scroll: &mut usize,
) -> SelectState {
    drawn_list(chosen_label(view), list, view.options.len(), scroll)
}

/// A select of `len` options showing `chosen`, with the list its Presenter holds, as the shared
/// dropdown draws it: [`drawn`] for a select that isn't a [`SelectView`], as a Settings field.
pub(crate) fn drawn_list(
    chosen: Option<String>,
    list: SelectList,
    len: usize,
    scroll: &mut usize,
) -> SelectState {
    let state = SelectState::drawn(chosen, list.highlight(), *scroll, len);
    *scroll = state.scroll();
    state
}
