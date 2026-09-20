//! The `?` help overlay: `dialog`'s chrome around `help::sections`' rows, read-only.

use gpui::{AnyElement, div, prelude::*, px};

use crate::{dialog, help::Section, theme::color};

const WIDTH: gpui::Pixels = px(520.0);
const KEY_COLUMN_WIDTH: gpui::Pixels = px(130.0);
const BODY_MAX_HEIGHT: gpui::Pixels = px(480.0);

pub fn render(sections: &[Section]) -> AnyElement {
    let rows = sections.iter().map(section).collect::<Vec<_>>();
    let overlay = dialog::overlay(
        WIDTH,
        false,
        div()
            .flex()
            .flex_col()
            .child(dialog::header("Keyboard help", false))
            .child(
                div()
                    .id("help-body")
                    .max_h(BODY_MAX_HEIGHT)
                    .overflow_y_scroll()
                    .child(dialog::body(rows)),
            )
            .child(
                div()
                    .px(px(20.0))
                    .py(px(12.0))
                    .border_t(px(1.0))
                    .border_color(color::HAIRLINE)
                    .text_size(px(11.5))
                    .text_color(color::INK_SECONDARY)
                    .child("? or esc to close"),
            ),
    );
    // Keyboard input is already swallowed in `InputMode::Help`; this stops clicks reaching the rails behind.
    div()
        .id("help-occluder")
        .absolute()
        .inset_0()
        .occlude()
        .child(overlay)
        .into_any_element()
}

fn section(section: &Section) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .text_color(color::INK_TERTIARY)
                .child(section.title.to_uppercase()),
        )
        .children(section.entries.iter().map(|(keys, action)| {
            div()
                .flex()
                .gap(px(12.0))
                .text_size(px(12.5))
                .child(
                    div()
                        .w(KEY_COLUMN_WIDTH)
                        .flex_none()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_color(color::INK)
                        .child(keys.clone()),
                )
                .child(
                    div()
                        .flex_1()
                        .text_color(color::INK_SECONDARY)
                        .child(*action),
                )
        }))
        .into_any_element()
}
