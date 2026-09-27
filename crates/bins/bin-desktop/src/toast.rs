//! The Toast layer: `Shell`'s `lib_toast` stack drawn bottom-right over the window, per
//! `docs/toasts-design.md` "Placement and layout". Hand-rolled rather than gpui-component's
//! `Notification` (#310): one line per Toast -- leading bar, glyph, Message, muted `×N`, ✕ --
//! newest nearest the status line, with a muted `+N more` line above the stack. `Shell` draws it
//! as its last child, above dialogs, the palette and their scrim, so an outcome is never hidden
//! by the surface that caused it.

use std::{rc::Rc, time::Duration};

use gpui::{AnyElement, App, Window, div, prelude::*, px};
use lib_toast::{Toast, Toasts};

use crate::{
    statusline,
    theme::{color, type_scale},
};

/// How often `Shell`'s Toast clock advances the model. Finer than any lifetime needs, coarse
/// enough to cost nothing while no Toast is live.
pub const TICK: Duration = Duration::from_millis(250);

const MAX_WIDTH: gpui::Pixels = px(420.0);
const HEIGHT: gpui::Pixels = px(36.0);
const INSET_RIGHT: gpui::Pixels = px(16.0);
const GAP_ABOVE_STATUS_LINE: gpui::Pixels = px(12.0);
const GAP: gpui::Pixels = px(8.0);

/// A Toast's ✕, given its index in [`Toasts::visible`].
pub type OnDismiss = Rc<dyn Fn(usize, &mut Window, &mut App)>;
/// The pointer entering (`true`) or leaving the stack, which pauses the timers.
pub type OnHover = Rc<dyn Fn(bool, &mut Window, &mut App)>;

/// The stack, or `None` when there's nothing to draw.
pub fn render<S>(
    toasts: &Toasts<S>,
    on_dismiss: OnDismiss,
    on_hover: OnHover,
    cx: &App,
) -> Option<AnyElement> {
    let visible = toasts.visible();
    if visible.is_empty() {
        return None;
    }
    let more = toasts.more_count();

    Some(
        div()
            .id("toast-stack")
            .absolute()
            .right(INSET_RIGHT)
            .bottom(statusline::HEIGHT + GAP_ABOVE_STATUS_LINE)
            .flex()
            .flex_col()
            .items_end()
            .gap(GAP)
            .font_family(type_scale::FONT_FAMILY)
            .text_size(type_scale::BODY)
            .on_hover(move |hovered, window, cx| on_hover(*hovered, window, cx))
            .when(more > 0, |this| {
                this.child(
                    div()
                        .text_size(type_scale::META)
                        .text_color(color::muted(cx))
                        .child(crate::msg::desktop_toast_more(&more.to_string())),
                )
            })
            // `visible` is oldest first, and a column stacks downwards, so the newest lands
            // nearest the status line.
            .children(
                visible
                    .iter()
                    .enumerate()
                    .map(|(index, toast)| render_toast(index, toast, on_dismiss.clone(), cx)),
            )
            .into_any_element(),
    )
}

fn render_toast(index: usize, toast: &Toast, on_dismiss: OnDismiss, cx: &App) -> AnyElement {
    let mark = color::toast_mark(toast.kind(), cx);
    div()
        .flex()
        .items_center()
        .max_w(MAX_WIDTH)
        .h(HEIGHT)
        .bg(color::info_toast_background(cx))
        .text_color(color::info_toast_text(cx))
        .border_1()
        .border_color(color::info_toast_border(cx))
        .overflow_hidden()
        .child(div().w(px(3.0)).h_full().flex_none().bg(mark))
        .child(
            div()
                .flex_none()
                .pl(px(10.0))
                .pr(px(8.0))
                .font_weight(type_scale::WEIGHT_BOLD)
                .text_color(mark)
                .child(toast.kind().glyph().to_string()),
        )
        // Shrinks and truncates before the badge or ✕ ever does.
        .child(
            div()
                .min_w(px(0.0))
                .truncate()
                .child(toast.text().to_string()),
        )
        .children((toast.count() > 1).then(|| {
            div()
                .flex_none()
                .pl(px(6.0))
                .text_color(color::muted(cx))
                .child(format!("×{}", toast.count()))
        }))
        .child(
            div()
                .id(("toast-dismiss", index))
                .flex_none()
                .h_full()
                .flex()
                .items_center()
                .px(px(10.0))
                .text_color(color::muted(cx))
                .cursor_pointer()
                .hover(|this| this.bg(color::hover(cx)))
                .on_click(move |_event, window, cx| on_dismiss(index, window, cx))
                .child("✕"),
        )
        .into_any_element()
}
