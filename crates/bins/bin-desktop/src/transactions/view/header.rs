//! The 4a view header (`docs/ux/desktop-mockups/03-transactions/README.md`): the title with its computed count
//! line and the **add transaction** button, then the filter chip row with `clear filters` and the
//! search box, closed by the 2px rule. Every click is the page Presenter's event, sent through
//! [`PageClicks`], so keys and clicks reach the Presenter alike.
//!
//! Chips follow the map's decision: an outline chip (at its default) ends in `▾`, an accent chip
//! (changed from the default) ends in a clickable `✕` that resets just that filter, and a click on
//! the chip itself opens the filter popover.

use crate::navigation::nav::NounLabel;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use gpui::{
    AnyElement, App, Bounds, ClickEvent, Pixels, SharedString, Window, canvas, div, prelude::*, px,
};

use lib_transactions::presenter::{
    ChipField, ChipView, TransactionsPageEvent, TransactionsPageViewModel,
};

use super::page::TransactionsPage;
use crate::{navigation::nav::Noun, shell::view::page_clicks::PageClicks, theme::color};

/// Where each chip was last painted, in window coordinates: written while painting, read by
/// `Root` to anchor the filter popover under the chip that opened it.
pub type ChipBounds = Rc<RefCell<HashMap<ChipField, Bounds<Pixels>>>>;

/// `padding:16px 28px 14px; border-bottom:2px solid rgba(32,30,29,.38)`. Each chip reports where
/// it was painted into `chip_bounds`.
pub fn render(
    view: &TransactionsPageViewModel,
    chip_bounds: &ChipBounds,
    clicks: &PageClicks<TransactionsPage>,
    cx: &App,
) -> AnyElement {
    div()
        .flex_none()
        .flex()
        .flex_col()
        .gap(px(12.0))
        .px(px(28.0))
        .pt(px(16.0))
        .pb(px(14.0))
        .border_b(px(2.0))
        .border_color(color::structural_rule(cx))
        .child(title_row(
            view.count_line.clone(),
            clicks.send(TransactionsPageEvent::New),
            cx,
        ))
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(8.0))
                .children(view.chips.iter().enumerate().map(|(index, chip_data)| {
                    chip(index, chip_data, clicks, chip_bounds.clone(), cx)
                }))
                .when(view.filters_changed, |this| {
                    this.child(clear_link(
                        clicks.send(TransactionsPageEvent::ClearFilters),
                        cx,
                    ))
                })
                .child(div().flex_1())
                .child(search_box(
                    &view.search,
                    view.searching,
                    clicks.send(TransactionsPageEvent::StartSearch),
                    cx,
                )),
        )
        .into_any_element()
}

/// The title (26px/800) with the count line (11.5px, tertiary) beside it, and the primary button.
fn title_row(
    count_line: String,
    on_add_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
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
                        .text_color(color::foreground(cx))
                        .child(Noun::Transactions.label()),
                )
                .child(
                    div()
                        .text_size(px(11.5))
                        .text_color(color::faint_text(cx))
                        .child(count_line),
                ),
        )
        .child(add_button(on_add_click, cx))
}

/// The primary button: 32px tall, ink fill, with the `n` key hint trailing at 75% opacity.
fn add_button(
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let hover = color::muted(cx);
    div()
        .id("transactions-add")
        .debug_selector(|| "transactions-add".to_string())
        .cursor_pointer()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(8.0))
        .h(px(32.0))
        .px(px(14.0))
        .bg(color::foreground(cx))
        .text_color(color::selection_text(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .whitespace_nowrap()
        .hover(move |style| style.bg(hover))
        .on_click(on_click)
        .child(crate::msg::desktop_transactions_add_button())
        .child(
            div()
                .opacity(0.75)
                .text_size(px(11.0))
                .font_weight(gpui::FontWeight::NORMAL)
                .child("n"),
        )
}

/// A filter chip (`.tag`, `padding:2px 8px`): outline with a trailing `▾` at its default, accent
/// with a trailing clickable `✕` once changed. A click on the chip opens the popover on its field;
/// the `✕` stops the click reaching the chip, which would otherwise also open it.
fn chip(
    index: usize,
    chip: &ChipView,
    clicks: &PageClicks<TransactionsPage>,
    bounds_store: ChipBounds,
    cx: &App,
) -> AnyElement {
    let field = chip.field;
    let clear = clicks.send(TransactionsPageEvent::ClearChip(field));
    let base = div()
        .id(("transactions-chip", index))
        .debug_selector(move || format!("transactions-chip-{index}"))
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
        .relative()
        .on_click(clicks.send(TransactionsPageEvent::OpenFilter(Some(field))))
        .child(SharedString::from(chip.label.clone()))
        // An invisible overlay whose only job is to report where this chip is painted.
        .child(
            canvas(
                move |bounds, _window, _cx| {
                    bounds_store.borrow_mut().insert(field, bounds);
                },
                |_bounds, _state, _window, _cx| {},
            )
            .absolute()
            .size_full(),
        );

    if chip.active {
        base.bg(color::accent_tint(cx))
            .border_color(color::accent(cx))
            .text_color(color::accent_tint_text(cx))
            .child(
                div()
                    .id(("transactions-chip-clear", index))
                    .debug_selector(move || format!("transactions-chip-clear-{index}"))
                    .cursor_pointer()
                    .opacity(0.85)
                    .on_click(move |event: &ClickEvent, window, cx| {
                        cx.stop_propagation();
                        clear(event, window, cx);
                    })
                    .child("\u{2715}"),
            )
            .into_any_element()
    } else {
        let hover = color::hover(cx);
        base.border_color(color::border(cx))
            .text_color(color::muted(cx))
            .hover(move |style| style.bg(hover))
            .child(div().text_size(px(9.0)).child("\u{25be}"))
            .into_any_element()
    }
}

/// The `clear filters` link: 11.5px, underlined.
fn clear_link(
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    div()
        .id("transactions-clear-filters")
        .debug_selector(|| "transactions-clear-filters".to_string())
        .cursor_pointer()
        .flex_none()
        .text_size(px(11.5))
        .text_color(color::foreground(cx))
        .underline()
        .on_click(on_click)
        .child(crate::msg::desktop_transactions_clear_filters())
}

/// The right-aligned search box (28px tall, 12px): the `/ search payee or memo` placeholder, or the
/// text typed so far, with a caret and an accent border while search mode is active.
fn search_box(
    search: &str,
    searching: bool,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    cx: &App,
) -> impl IntoElement {
    let caret = if searching { "\u{2502}" } else { "" };
    let (text, text_color) = if search.is_empty() && !searching {
        (
            SharedString::from(crate::msg::desktop_transactions_search_placeholder()),
            color::faint_text(cx),
        )
    } else {
        (
            SharedString::from(format!("{search}{caret}")),
            color::foreground(cx),
        )
    };
    div()
        .id("transactions-search")
        .debug_selector(|| "transactions-search".to_string())
        .cursor_pointer()
        .flex_none()
        .flex()
        .items_center()
        .w(px(240.0))
        .h(px(28.0))
        .px(px(10.0))
        .border_1()
        .border_color(if searching {
            color::accent(cx)
        } else {
            color::border(cx)
        })
        .text_size(px(12.0))
        .text_color(text_color)
        .truncate()
        .on_click(on_click)
        .child(text)
}
