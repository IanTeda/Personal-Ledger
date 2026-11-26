//! The field helpers the Accounts Add dialog defines and the other Dialogs reuse: the two-up row,
//! the label and its optional or suffixed forms, and the text box. They are the shared shape of
//! the Settings and Ledger dialogs' fields, so they live in `form/` rather than in one page.

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};

use super::caret;
use gpui_component::Sizable as _;

use crate::assets::icon::DesktopIcon;
use crate::{dialog, theme::color};

/// Two fields side by side: `gap:16px`, each `flex:1`. Each column takes half the row from a zero
/// basis, so a field is as wide empty as it is filled, rather than growing with its text.
pub(crate) fn two_up(fields: [AnyElement; 2]) -> AnyElement {
    div()
        .flex()
        .items_start()
        .gap(px(16.0))
        .children(
            fields
                .into_iter()
                .map(|field| div().flex_1().min_w(px(0.0)).child(field)),
        )
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

/// A label above a clickable text box whose border turns `accent` with a blinking caret while
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
    div()
        .child(label)
        .child(
            text_box(id, focused, cx)
                .on_click(move |_event, window, cx| on_click(window, cx))
                .child(caret::field_text(value, placeholder, focused, cx)),
        )
        .into_any_element()
}

/// What a number field's clicks send: a click on its box, and on its up and down arrows.
pub(crate) struct StepperClicks {
    pub on_click: dialog::OnClick,
    pub on_up: dialog::OnClick,
    pub on_down: dialog::OnClick,
}

/// [`text_field`] for a small whole number, with up and down arrows stacked at the box's right
/// edge: the Precision field. Each arrow is its own click target, `<id>-up` and `<id>-down`, so a
/// click on one steps the number rather than only focusing the box.
pub(crate) fn stepper_field(
    id: &'static str,
    label: AnyElement,
    value: &str,
    placeholder: &str,
    focused: bool,
    clicks: StepperClicks,
    cx: &App,
) -> AnyElement {
    let StepperClicks {
        on_click,
        on_up,
        on_down,
    } = clicks;
    let (muted, ink) = (color::muted(cx), color::foreground(cx));
    let arrow = |which: &'static str, icon: DesktopIcon, on_click: dialog::OnClick| {
        let arrow_id = format!("{id}-{which}");
        div()
            .id(SharedString::from(arrow_id.clone()))
            .debug_selector(move || arrow_id.clone())
            .cursor_pointer()
            .flex()
            .items_center()
            .justify_center()
            .w(px(18.0))
            .h(px(14.0))
            .text_color(muted)
            .hover(move |style| style.text_color(ink))
            .on_click(move |_event, window, cx| {
                cx.stop_propagation();
                on_click(window, cx);
            })
            .child(icon.icon().with_size(px(14.0)))
    };
    div()
        .child(label)
        .child(
            text_box(id, focused, cx)
                .flex()
                .items_center()
                .justify_between()
                .on_click(move |_event, window, cx| on_click(window, cx))
                .child(caret::field_text(value, placeholder, focused, cx))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_none()
                        // Into the box's padding, so the two arrows keep it a text box's height.
                        .my(px(-6.0))
                        .child(arrow("up", DesktopIcon::ChevronUp, on_up))
                        .child(arrow("down", DesktopIcon::ChevronDown, on_down)),
                ),
        )
        .into_any_element()
}

/// The clickable box a text field draws in: `padding:8px 10px`, its border `accent` while focused.
fn text_box(id: &'static str, focused: bool, cx: &App) -> gpui::Stateful<gpui::Div> {
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
}
