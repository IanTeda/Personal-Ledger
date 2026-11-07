//! The Units Dialogs' Views: each `lib-units` Dialog Presenter's `DialogView` (one file each in
//! `dialog/`), which the generic `ModuleDialog` hosts in the Dialog host (ADR-0056). Every Dialog
//! command and typed key reaches the Presenter as a `lib-viper` `DialogInput`, and every click as
//! a Presenter event; each View draws the Presenter's view model, open lists included, and
//! nothing else.
//!
//! Delete unit is the Unit Deletion module's Dialog (`crate::unit_deletion`), which the page's
//! Router opens instead, since it writes the Institutions too.

mod delete_price_source;
mod price_source_form;
mod set_base_unit;
mod set_price;
mod unit_form;

use gpui::{AnyElement, App, div, prelude::*, px};

use crate::form::viper::{chosen_label, drawn, labels};
use crate::theme::color;

/// Why the last confirm was refused, in the danger colour under the fields.
pub(crate) fn refusal(text: Option<String>, cx: &App) -> Option<AnyElement> {
    text.map(|text| {
        div()
            .debug_selector(|| "units-dialog-refusal".to_string())
            .text_size(px(12.0))
            .text_color(color::accent_text(cx))
            .child(text)
            .into_any_element()
    })
}

/// A field's 11.5px muted help line under it.
pub(crate) fn help(text: String, cx: &App) -> AnyElement {
    div()
        .mt(px(6.0))
        .text_size(px(11.5))
        .text_color(color::muted(cx))
        .child(text)
        .into_any_element()
}

/// A test button that has nothing behind it yet: drawn disabled beside its note.
fn test_not_built(id: &'static str, label: String, note: String, cx: &App) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(px(10.0))
        .child(
            div()
                .debug_selector(move || id.to_string())
                .py(px(7.0))
                .px(px(12.0))
                .border_1()
                .border_color(color::border(cx))
                .opacity(0.45)
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(12.0))
                .child(label),
        )
        .child(
            div()
                .text_size(px(11.5))
                .text_color(color::faint_text(cx))
                .child(note),
        )
        .into_any_element()
}
