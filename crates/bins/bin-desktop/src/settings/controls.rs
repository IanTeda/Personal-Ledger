//! What General and Display draw their controls with: the keyboard's focus mark, the line under a
//! control and the note when a source above the configuration file sets its key.

use gpui::{App, div, prelude::*, px};
use lib_core::configuration::ConfigKey;

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
