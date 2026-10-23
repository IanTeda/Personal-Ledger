//! The placeholder a View shows in place of a page it can't build: a noun whose surface is a
//! later map's work, or a page whose record has gone.

use crate::navigation::nav::NounLabel;
use gpui::{AnyElement, App, div, prelude::*, px};

use crate::{navigation::nav::Noun, theme::color};

/// The "not yet built" view of a noun whose surface is a later map's work (#420): its name and
/// one line saying so. The rail row and `g` binding are live; the surface is not.
pub fn render(noun: Noun, cx: &App) -> AnyElement {
    div()
        .debug_selector(|| "placeholder-view".to_string())
        .p(px(24.0))
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(18.0))
                .child(crate::msg::desktop_placeholder_title(&noun.label())),
        )
        .child(
            div()
                .text_color(color::faint_text(cx))
                .child(crate::msg::desktop_placeholder_body()),
        )
        .into_any_element()
}
