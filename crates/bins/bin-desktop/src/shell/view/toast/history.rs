//! The session Toast history (`docs/toasts-design.md` "Session Toast history", #339): a modal
//! card on `dialog`'s frame listing every Toast raised this run, newest first -- Kind glyph in
//! its mark colour, the time last raised, the full untruncated Message and its `×N`. The list
//! scrolls; `Esc` or Close shuts it, and viewing dismisses nothing.

use std::rc::Rc;

use chrono::{DateTime, Local};
use gpui::{
    AnyElement, App, Context, EventEmitter, IntoElement, Render, Window, div, prelude::*, px,
};
use lib_shell::{ToastHistoryPresenter, ToastHistoryRow};
use lib_toast::History;
use lib_viper::dialog::DialogInput;

use crate::{
    dialog,
    shell::view::dialog_entity::{
        DialogClicks, DialogClosed, DialogReadback, HostRouter, HostedDialog,
    },
    theme::{color, type_scale},
};

const WIDTH: gpui::Pixels = px(560.0);
/// Past this the list scrolls rather than the card outgrowing a small window.
const LIST_MAX_HEIGHT: gpui::Pixels = px(420.0);

pub type OnClose = dialog::OnClick;

/// The session Toast history: an Entity holding `lib-shell`'s [`ToastHistoryPresenter`], hosted
/// like any module Dialog (#1043). Read-only, so `Esc` and the Close button close it and every
/// key is left to the Shell.
pub struct ToastHistoryDialog {
    presenter: ToastHistoryPresenter,
}

impl ToastHistoryDialog {
    pub fn new(presenter: ToastHistoryPresenter) -> Self {
        Self { presenter }
    }

    /// Shows a Toast raised while the card is up.
    pub fn refresh(&mut self, history: &History<DateTime<Local>>, cx: &mut Context<'_, Self>) {
        self.presenter.refresh(history);
        cx.notify();
    }

    /// Emits the close once the Presenter has closed it.
    fn closed(router: HostRouter<()>, cx: &mut Context<'_, Self>) {
        if router.closed().is_some() {
            cx.emit(DialogClosed);
        }
    }
}

impl EventEmitter<DialogClosed> for ToastHistoryDialog {}

impl HostedDialog for ToastHistoryDialog {
    /// Every key is left to the Shell, so none reaches the Presenter.
    fn input(&mut self, _input: DialogInput, _cx: &mut Context<'_, Self>) {}

    fn can_confirm(&self) -> bool {
        true
    }

    /// The Close button.
    fn confirm(&mut self, cx: &mut Context<'_, Self>) {
        let mut router = HostRouter::default();
        self.presenter.confirm(&mut router);
        Self::closed(router, cx);
    }

    fn cancel(&mut self, cx: &mut Context<'_, Self>) {
        let mut router = HostRouter::default();
        self.presenter.cancel(&mut router);
        Self::closed(router, cx);
    }

    fn close_open_select(&mut self) -> bool {
        false
    }

    fn readback(&self, _cx: &App) -> DialogReadback {
        DialogReadback {
            kind: "ToastHistory",
            values: vec![("rows", self.presenter.view_model().rows.len().to_string())],
        }
    }
}

impl Render for ToastHistoryDialog {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let clicks = DialogClicks::new(cx);
        render(&self.presenter.view_model().rows, clicks.confirm, cx)
    }
}

fn render(rows: &[ToastHistoryRow], on_close: OnClose, cx: &App) -> AnyElement {
    let list = if rows.is_empty() {
        div()
            .text_color(color::muted(cx))
            .child(lib_locale::msg::toast_history_empty())
            .into_any_element()
    } else {
        div()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .children(rows.iter().map(|row| render_row(row, cx)))
            .into_any_element()
    };

    let overlay = dialog::overlay(
        WIDTH,
        false,
        div()
            .flex()
            .flex_col()
            .font_family(type_scale::FONT_FAMILY)
            .child(dialog::header(
                lib_locale::msg::toast_history_title(),
                false,
                cx,
            ))
            .child(
                div()
                    .id("toast-history-list")
                    .max_h(LIST_MAX_HEIGHT)
                    .overflow_y_scroll()
                    .p(px(20.0))
                    .text_size(type_scale::BODY)
                    .child(list),
            )
            .child(dialog::action_row(
                [dialog::confirm_button(
                    "toast-history-close",
                    lib_locale::msg::dialog_close(),
                    true,
                    false,
                    Rc::clone(&on_close),
                    cx,
                )
                .into_any_element()],
                cx,
            )),
        cx,
    );
    // Keyboard input is already swallowed in `InputMode::Dialog`; this stops clicks reaching the
    // rails behind.
    div()
        .id("toast-history-occluder")
        .absolute()
        .inset_0()
        .occlude()
        .child(overlay)
        .into_any_element()
}

fn render_row(row: &ToastHistoryRow, cx: &App) -> AnyElement {
    div()
        .flex()
        .items_start()
        .gap(px(10.0))
        .child(
            div()
                .flex_none()
                .w(px(12.0))
                .font_weight(type_scale::WEIGHT_BOLD)
                .text_color(color::toast_mark(row.kind, cx))
                .child(row.kind.glyph().to_string()),
        )
        .child(
            div()
                .flex_none()
                .text_size(type_scale::META)
                .text_color(color::muted(cx))
                .child(row.time.clone()),
        )
        // Wraps rather than truncating: the history is where a long Message is read in full.
        .child(div().flex_1().min_w(px(0.0)).child(row.text.clone()))
        .children(
            row.badge
                .clone()
                .map(|badge| div().flex_none().text_color(color::muted(cx)).child(badge)),
        )
        .into_any_element()
}
