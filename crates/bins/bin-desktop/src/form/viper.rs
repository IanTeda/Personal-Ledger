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
    let state = SelectState::drawn(
        chosen_label(view),
        list.highlight(),
        *scroll,
        view.options.len(),
    );
    *scroll = state.scroll();
    state
}
