//! The `?` help overlay: an About-style card on `dialog`'s chrome -- project facts on the left,
//! the shortcut cheat-sheet on the right, a Close button in the footer.

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};

use crate::{dialog, help, theme::color};

const WIDTH: gpui::Pixels = px(720.0);
const FACTS_WIDTH: gpui::Pixels = px(340.0);
const KEY_WIDTH: gpui::Pixels = px(44.0);

pub type OnClose = dialog::OnClick;

pub fn render(on_close: OnClose) -> AnyElement {
    let overlay = dialog::overlay(
        WIDTH,
        false,
        div()
            .flex()
            .flex_col()
            .child(header())
            .child(div().flex().child(facts_column()).child(shortcuts_column()))
            .child(footer(on_close)),
    );
    // Keyboard input is already swallowed in `InputMode::Help`; this stops clicks reaching the
    // rails behind.
    div()
        .id("help-occluder")
        .absolute()
        .inset_0()
        .occlude()
        .child(overlay)
        .into_any_element()
}

fn header() -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(14.0))
        .px(px(24.0))
        .py(px(18.0))
        .border_b(px(2.0))
        .border_color(color::BORDER)
        .child(
            div()
                .flex_none()
                .size(px(26.0))
                .border(px(2.0))
                .border_color(color::INK)
                .child(div().ml(px(6.0)).w(px(2.0)).h_full().bg(color::INK)),
        )
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(19.0))
                .child(lib_locale::msg::app_name()),
        )
        .child(
            div()
                .text_size(px(11.5))
                .text_color(color::INK_TERTIARY)
                .child(format!(
                    "v{} \u{b7} {}",
                    env!("CARGO_PKG_VERSION"),
                    help::CHANNEL
                )),
        )
        .child(div().flex_1())
        .child(
            div()
                .text_size(px(12.0))
                .text_color(color::INK_TERTIARY)
                .child(lib_locale::msg::nav_help()),
        )
}

fn kicker(text: impl Into<SharedString>) -> impl IntoElement {
    div()
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.5))
        .text_color(color::INK_SECONDARY)
        .child(lib_locale::format::upper(&text.into()))
}

fn link(id: &'static str, text: impl Into<SharedString>, url: &'static str) -> impl IntoElement {
    div()
        .id(id)
        .cursor_pointer()
        .text_color(color::ACCENT)
        .on_click(move |_event, _window: &mut Window, cx: &mut App| cx.open_url(url))
        .child(text.into())
}

fn fact(kicker_text: impl Into<SharedString>, value: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(6.0))
        .child(kicker(kicker_text))
        .child(value)
}

/// The licence line: the Message's `<license>` span is the link, the rest is plain text.
fn license() -> impl IntoElement {
    div()
        .flex()
        .flex_wrap()
        .children(
            crate::msg::desktop_help_license()
                .into_iter()
                .map(|segment| match segment.tag.as_deref() {
                    Some("license") => {
                        link("license", segment.text, help::LICENSE_URL).into_any_element()
                    }
                    _ => div().child(segment.text).into_any_element(),
                }),
        )
}

fn rule() -> impl IntoElement {
    div().h(px(1.0)).bg(color::HAIRLINE)
}

fn facts_column() -> impl IntoElement {
    div()
        .w(FACTS_WIDTH)
        .flex_none()
        .p(px(24.0))
        .child(facts(None, None))
}

/// The project facts (description, author, links, licence, copyright), shared with Settings'
/// About section so the two never drift apart. `before_author`/`after_author` slot extra rows
/// in around the Author fact.
pub fn facts(
    before_author: Option<AnyElement>,
    after_author: Option<AnyElement>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(14.0))
        .text_size(px(12.5))
        .child(
            div()
                .text_color(color::INK_SECONDARY)
                .child(help::description()),
        )
        .child(rule())
        .children(before_author)
        .child(fact(
            crate::msg::desktop_help_author_label(),
            div().text_color(color::ACCENT).child(help::AUTHOR),
        ))
        .children(after_author)
        .children(help::LINKS.iter().map(|item| {
            fact((item.label)(), link(item.id, item.text, item.url)).into_any_element()
        }))
        .child(fact(crate::msg::desktop_help_license_label(), license()))
        .child(rule())
        .child(
            div()
                .text_size(px(11.5))
                .text_color(color::INK_TERTIARY)
                .child(help::copyright()),
        )
}

fn shortcut(label: impl Into<SharedString>, keys: impl Into<SharedString>) -> AnyElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap(px(8.0))
        .child(div().text_color(color::INK_SECONDARY).child(label.into()))
        .child(
            div()
                .min_w(KEY_WIDTH)
                .flex()
                .justify_end()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(color::INK)
                .child(keys.into()),
        )
        .into_any_element()
}

fn two_columns(cells: Vec<AnyElement>) -> impl IntoElement {
    let mut rows = Vec::new();
    let mut cells = cells.into_iter();
    while let Some(left) = cells.next() {
        let right = cells.next();
        rows.push(
            div()
                .flex()
                .gap(px(32.0))
                .child(div().flex_1().child(left))
                .child(div().flex_1().children(right))
                .into_any_element(),
        );
    }
    div().flex().flex_col().gap(px(10.0)).children(rows)
}

fn shortcuts_column() -> impl IntoElement {
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .p(px(24.0))
        .border_l(px(1.0))
        .border_color(color::HAIRLINE)
        .text_size(px(12.5))
        .child(kicker(crate::msg::desktop_help_shortcuts_label()))
        .child(two_columns(
            help::jump_shortcuts()
                .into_iter()
                .map(|(label, keys)| shortcut(label, keys))
                .collect(),
        ))
        .child(rule())
        .child(two_columns(
            help::global_shortcuts()
                .into_iter()
                .map(|(label, keys)| shortcut(label, keys))
                .collect(),
        ))
}

fn footer(on_close: OnClose) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .px(px(24.0))
        .py(px(16.0))
        .border_t(px(1.0))
        .border_color(color::HAIRLINE)
        .child(
            div()
                .text_size(px(11.5))
                .text_color(color::INK_TERTIARY)
                .child(crate::msg::desktop_help_footer_note()),
        )
        .child(dialog::confirm_button(
            "help-close",
            lib_locale::msg::dialog_close(),
            true,
            false,
            Rc::clone(&on_close),
        ))
}
