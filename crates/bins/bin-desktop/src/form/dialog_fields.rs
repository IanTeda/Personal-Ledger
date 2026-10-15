//! The field helpers the Accounts Add dialog defines and the other Dialogs reuse: the two-up row,
//! the label and its optional or suffixed forms, and the text box. They are the shared shape of
//! the Settings and Ledger dialogs' fields, so they live in `form/` rather than in one page.

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};

use crate::{dialog, theme::color};

/// Two fields side by side: `gap:16px`, each `flex:1`.
pub(crate) fn two_up(fields: [AnyElement; 2]) -> AnyElement {
    div()
        .flex()
        .items_start()
        .gap(px(16.0))
        .children(fields)
        .into_any_element()
}

/// A field label: `font-weight:800; font-size:12px; margin-bottom:6px`.
pub(crate) fn label(text: impl Into<SharedString>) -> AnyElement {
    div()
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(12.0))
        .mb(px(6.0))
        .child(text.into())
        .into_any_element()
}

/// A label suffixed `(optional)` in the tertiary ink.
pub(crate) fn optional_label(text: impl Into<SharedString>, cx: &App) -> AnyElement {
    suffixed_label(text, crate::msg::desktop_field_optional(), cx)
}

/// A label followed by a `suffix` in the tertiary ink, at regular weight.
pub(crate) fn suffixed_label(
    text: impl Into<SharedString>,
    suffix: impl Into<SharedString>,
    cx: &App,
) -> AnyElement {
    div()
        .flex()
        .gap(px(4.0))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(12.0))
        .mb(px(6.0))
        .child(text.into())
        .child(
            div()
                .font_weight(gpui::FontWeight::NORMAL)
                .text_color(color::faint_text(cx))
                .child(suffix.into()),
        )
        .into_any_element()
}

/// A label above a clickable text box whose border turns `accent` with a trailing caret while
/// focused -- the same look as the Settings dialogs' text fields.
pub(crate) fn text_field(
    id: &'static str,
    label: AnyElement,
    value: &str,
    placeholder: &str,
    focused: bool,
    on_click: dialog::OnClick,
    cx: &App,
) -> AnyElement {
    let caret = if focused { "\u{2502}" } else { "" };
    let (text, text_color) = if value.is_empty() {
        (
            SharedString::from(format!("{placeholder}{caret}")),
            color::faint_text(cx),
        )
    } else {
        (
            SharedString::from(format!("{value}{caret}")),
            color::foreground(cx),
        )
    };

    div()
        .child(label)
        .child(
            div()
                .id(id)
                .debug_selector(move || id.to_string())
                .cursor_pointer()
                .w_full()
                .py(px(8.0))
                .px(px(10.0))
                .border_1()
                .border_color(if focused {
                    color::accent(cx)
                } else {
                    color::border(cx)
                })
                .text_size(px(13.0))
                .text_color(text_color)
                .on_click(move |_event, window, cx| on_click(window, cx))
                .child(text),
        )
        .into_any_element()
}
