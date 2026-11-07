//! The Inventory Dialogs' Views: each `lib-inventory` Dialog Presenter's `DialogView` (one file
//! each in `dialog/`), which the generic `ModuleDialog` hosts in the Dialog host (ADR-0056). Every
//! Dialog command and typed key reaches the Presenter as a `lib-viper` `DialogInput`, which it
//! reads itself, and every click becomes a Presenter event; each View draws the Presenter's view
//! model, hints included, and nothing else.

mod area_blocked;
mod area_form;
mod location_form;
mod remove_area;
mod remove_location;

use gpui::{App, IntoElement, div, prelude::*, px};

use crate::theme::color;

/// A line of help or error text under a field: muted, or in the accent colour for an error.
fn note(text: String, error: bool, cx: &App) -> impl IntoElement {
    div()
        .mt(px(4.0))
        .text_size(px(11.5))
        .text_color(if error {
            color::accent_text(cx)
        } else {
            color::muted(cx)
        })
        .child(text)
}
