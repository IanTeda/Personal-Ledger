//! What General and Display draw their controls with: the keyboard's focus mark, a field's
//! segmented bar, the line under a control and the note when a source above the configuration file
//! sets its key.

use gpui::{App, SharedString, div, prelude::*, px};
use lib_core::configuration::ConfigKey;
use lib_settings::presenter::FieldView;

use crate::form::fields::{OnSegmentClick, segmented_labelled};
use crate::theme::color;

/// The keyboard-focus mark for one control: an accent rule down its left edge, drawn on every
/// control so the column does not shift as focus moves.
pub(super) fn focus_mark(focused: bool, cx: &App) -> gpui::Div {
    div()
        .pl(px(8.0))
        .ml(px(-10.0))
        .border_l(px(2.0))
        .border_color(if focused {
            gpui::Hsla::from(color::accent(cx))
        } else {
            gpui::transparent_black()
        })
}

/// A field's choices as a segmented bar, the one chosen filled (the first while its value isn't a
/// choice). A click on an option hands `on_click` its index.
pub(super) fn segmented_choices(
    id: &'static str,
    view: &FieldView,
    on_click: OnSegmentClick<usize>,
    option_width: Option<gpui::Pixels>,
    cx: &App,
) -> impl IntoElement {
    let options = view
        .options
        .iter()
        .enumerate()
        .map(|(index, label)| (index, SharedString::from(label.clone())))
        .collect();
    segmented_labelled(
        id,
        options,
        view.chosen.unwrap_or(0),
        on_click,
        option_width,
        cx,
    )
}

/// The line under a control.
pub(super) fn hint(text: String, cx: &App) -> impl IntoElement {
    div()
        .mt(px(3.0))
        .text_size(px(12.0))
        .text_color(color::muted(cx))
        .child(text)
}

/// The note under `key`'s control while a source above the file sets it, so the control says why
/// a change won't take.
pub(super) fn shadowed(key: ConfigKey, note: Option<&str>, cx: &App) -> Option<gpui::Div> {
    let note = note?.to_string();
    Some(
        div()
            .debug_selector(move || format!("shadowed-{}", key.name()))
            .mt(px(3.0))
            .text_size(px(12.0))
            .text_color(color::muted(cx))
            .child(note),
    )
}
