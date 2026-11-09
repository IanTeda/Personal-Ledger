//! The Budget form (11c), Edit budget (9e), Stop budgeting (9g), Fill (9f) and the Category detail
//! (9d): each `lib-budgets` Presenter's `DialogView` (`form_dialog/`), which the generic
//! `ModuleDialog` hosts in the Dialog host (ADR-0056), with the drawing helpers they share. Every
//! Dialog command and typed key reaches the Presenter as a `lib-viper` `DialogInput`, which it
//! reads itself, every click is a Presenter event, and each View draws its view model, hints
//! included. A confirmed outcome lands on the page, which opens what follows (Edit budget from the
//! detail, Stop from Edit budget, the new Budget on show).

mod budget;
mod detail;
mod fill;
mod limit;
mod stop;

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};

use crate::dialog;
use crate::theme::color;

/// A labelled `.seg`: the chosen cell filled dark, an accent border while focused. Each cell is
/// its text, whether it is the chosen one, and its click.
pub(super) fn segmented(
    id: &'static str,
    title: String,
    focused: bool,
    cells: Vec<(String, bool, dialog::OnClick)>,
    cx: &App,
) -> AnyElement {
    div()
        .child(crate::form::dialog_fields::label(title))
        .child(
            div()
                .flex()
                .border_1()
                .border_color(if focused {
                    color::accent(cx)
                } else {
                    color::border(cx)
                })
                .children(cells.into_iter().enumerate().map(
                    |(index, (text, chosen, on_click))| {
                        div()
                            .id(SharedString::from(format!("{id}-{index}")))
                            .debug_selector(move || format!("{id}-{index}"))
                            .cursor_pointer()
                            .flex_1()
                            .py(px(8.0))
                            .px(px(10.0))
                            .text_size(px(12.5))
                            .text_align(gpui::TextAlign::Center)
                            .font_weight(gpui::FontWeight::EXTRA_BOLD)
                            .when(chosen, |this| {
                                this.bg(color::foreground(cx))
                                    .text_color(color::background(cx))
                            })
                            .on_click(move |_event, window, cx| on_click(window, cx))
                            .child(text)
                    },
                )),
        )
        .into_any_element()
}

/// A line of help text under a field.
pub(super) fn help(text: String, cx: &App) -> AnyElement {
    div()
        .mt(px(6.0))
        .text_size(px(11.5))
        .text_color(color::muted(cx))
        .child(text)
        .into_any_element()
}

/// A click on a cell that picks `value` of `T`, as one input.
pub(super) fn picker<T: Copy + 'static>(
    choose: crate::shell::view::dialog_entity::OnChoose<T>,
    value: T,
) -> dialog::OnClick {
    Rc::new(move |window: &mut Window, cx: &mut App| choose(value, window, cx))
}
