//! The 4a view header (`docs/ux/desktop/Transactions/README.md`): the title with its computed count
//! line and the **add transaction** button, then the filter chip row with `clear filters` and the
//! search box, closed by the 2px rule. Every action is a callback into `Shell`, so keys and clicks
//! share handlers.
//!
//! Chips follow the map's decision: an outline chip (at its default) ends in `▾`, an accent chip
//! (changed from the default) ends in a clickable `✕` that resets just that filter, and a click on
//! the chip itself opens the filter popover.

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};

use crate::{
    theme::color,
    transaction_chips::{Chip, FilterField},
};

pub type OnPlainClick = Rc<dyn Fn(&mut Window, &mut App)>;
pub type OnChipClick = Rc<dyn Fn(FilterField, &mut Window, &mut App)>;

pub struct HeaderProps {
    /// `700 transactions across 4 accounts`.
    pub count_line: String,
    pub chips: Vec<Chip>,
    /// Whether the filters differ from the defaults, so `clear filters` has something to do.
    pub show_clear: bool,
    /// The search text.
    pub search: String,
    /// Whether the global search mode is active on this page (the box shows a caret).
    pub searching: bool,
    pub on_add_click: OnPlainClick,
    /// A chip, or its `▾`: opens the filter popover on that chip's field.
    pub on_chip_click: OnChipClick,
    /// The `✕` on an accent chip.
    pub on_chip_clear: OnChipClick,
    pub on_clear_all: OnPlainClick,
    /// A click on the search box: enters search mode.
    pub on_search_click: OnPlainClick,
}

/// `padding:16px 28px 14px; border-bottom:2px solid rgba(32,30,29,.38)`.
pub fn render(props: HeaderProps) -> AnyElement {
    let HeaderProps {
        count_line,
        chips,
        show_clear,
        search,
        searching,
        on_add_click,
        on_chip_click,
        on_chip_clear,
        on_clear_all,
        on_search_click,
    } = props;

    div()
        .flex_none()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .px(px(28.0))
        .pt(px(16.0))
        .pb(px(14.0))
        .border_b(px(2.0))
        .border_color(color::STRUCTURAL_RULE)
        .child(title_row(count_line, on_add_click))
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(8.0))
                .children(chips.into_iter().enumerate().map(|(index, chip_data)| {
                    chip(
                        index,
                        chip_data,
                        on_chip_click.clone(),
                        on_chip_clear.clone(),
                    )
                }))
                .when(show_clear, |this| this.child(clear_link(on_clear_all)))
                .child(div().flex_1())
                .child(search_box(search, searching, on_search_click)),
        )
        .into_any_element()
}

/// The title (26px/800) with the count line (11.5px, tertiary) beside it, and the primary button.
fn title_row(count_line: String, on_add_click: OnPlainClick) -> impl IntoElement {
    div()
        .flex()
        .items_end()
        .justify_between()
        .gap(px(16.0))
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_baseline()
                .gap(px(14.0))
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(26.0))
                        .text_color(color::INK)
                        .child("Transactions"),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(color::INK_TERTIARY)
                        .child(count_line),
                ),
        )
        .child(add_button(on_add_click))
}

/// The primary button: 32px tall, ink fill, with the `n` key hint trailing at 75% opacity.
fn add_button(on_click: OnPlainClick) -> impl IntoElement {
    div()
        .id("transactions-add")
        .cursor_pointer()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(8.0))
        .h(px(32.0))
        .px(px(14.0))
        .bg(color::INK)
        .text_color(color::INK_ON_DARK)
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .whitespace_nowrap()
        .hover(|style| style.bg(color::INK_SECONDARY))
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child("add transaction")
        .child(
            div()
                .opacity(0.75)
                .text_size(px(11.0))
                .font_weight(gpui::FontWeight::NORMAL)
                .child("n"),
        )
}

/// A filter chip (`.tag`, `padding:2px 8px`): outline with a trailing `▾` at its default, accent
/// with a trailing clickable `✕` once changed. The `✕` stops the click reaching the chip, which
/// would otherwise also open the popover.
fn chip(index: usize, chip: Chip, on_click: OnChipClick, on_clear: OnChipClick) -> AnyElement {
    let field = chip.field;
    let base = div()
        .id(("transactions-chip", index))
        .cursor_pointer()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(6.0))
        .px(px(8.0))
        .py(px(2.0))
        .border_1()
        .text_size(px(11.5))
        .whitespace_nowrap()
        .on_click(move |_event, window, cx| on_click(field, window, cx))
        .child(SharedString::from(chip.label));

    if chip.active {
        base.bg(color::TAG_ACCENT_BG)
            .border_color(color::ACCENT)
            .text_color(color::TAG_ACCENT_TEXT)
            .child(
                div()
                    .id(("transactions-chip-clear", index))
                    .cursor_pointer()
                    .opacity(0.85)
                    .on_click(move |_event, window, cx| {
                        cx.stop_propagation();
                        on_clear(field, window, cx)
                    })
                    .child("\u{2715}"),
            )
            .into_any_element()
    } else {
        base.border_color(color::BORDER)
            .text_color(color::INK_SECONDARY)
            .hover(|style| style.bg(color::HOVER_TINT))
            .child(div().text_size(px(9.0)).child("\u{25be}"))
            .into_any_element()
    }
}

/// The `clear filters` link: 11.5px, underlined.
fn clear_link(on_click: OnPlainClick) -> impl IntoElement {
    div()
        .id("transactions-clear-filters")
        .cursor_pointer()
        .flex_none()
        .text_size(px(11.5))
        .text_color(color::INK)
        .underline()
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child("clear filters")
}

/// The right-aligned search box (28px tall, 12px): the `/ search payee or memo` placeholder, or the
/// text typed so far, with a caret and an accent border while search mode is active.
fn search_box(search: String, searching: bool, on_click: OnPlainClick) -> impl IntoElement {
    let caret = if searching { "\u{2502}" } else { "" };
    let (text, text_color) = if search.is_empty() && !searching {
        (
            SharedString::from("/ search payee or memo"),
            color::INK_TERTIARY,
        )
    } else {
        (SharedString::from(format!("{search}{caret}")), color::INK)
    };
    div()
        .id("transactions-search")
        .cursor_pointer()
        .flex_none()
        .flex()
        .items_center()
        .w(px(240.0))
        .h(px(28.0))
        .px(px(10.0))
        .border_1()
        .border_color(if searching {
            color::ACCENT
        } else {
            color::BORDER
        })
        .text_size(px(12.0))
        .text_color(text_color)
        .truncate()
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(text)
}
