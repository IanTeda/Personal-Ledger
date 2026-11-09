//! The session Toast history (`docs/toasts-design.md` "Session Toast history", #339): a modal
//! card on `dialog`'s frame listing every Toast raised this run, newest first -- Kind glyph in
//! its mark colour, the time last raised, the full untruncated Message and its `×N`. The list
//! scrolls; `Esc` or Close shuts it, and viewing dismisses nothing.

use std::rc::Rc;

use gpui::{AnyElement, App, div, prelude::*, px};
use lib_shell::{Hint, ToastHistoryPresenter, ToastHistoryRow, ToastHistoryViewModel};

use crate::{
    dialog,
    shell::view::dialog_entity::{DialogView, ModuleDialogClicks},
    theme::{color, type_scale},
};

const WIDTH: gpui::Pixels = px(560.0);
/// Past this the list scrolls rather than the card outgrowing a small window.
const LIST_MAX_HEIGHT: gpui::Pixels = px(420.0);

pub type OnClose = dialog::OnClick;

/// The session Toast history: `lib-shell`'s [`ToastHistoryPresenter`] hosted in a `ModuleDialog`
/// (#1043). Read-only, so `Esc` and the Close button close it and every key is left to the Shell.
impl DialogView for ToastHistoryPresenter {
    type Widgets = ();

    fn kind(_view: &ToastHistoryViewModel) -> &'static str {
        "ToastHistory"
    }

    fn render(
        view: &ToastHistoryViewModel,
        _widgets: &mut (),
        clicks: &ModuleDialogClicks<Self>,
        cx: &App,
    ) -> AnyElement {
        render(&view.rows, clicks.confirm.clone(), cx)
    }

    fn readback(view: &ToastHistoryViewModel) -> Vec<(&'static str, String)> {
        vec![("rows", view.rows.len().to_string())]
    }

    /// The card is read-only and leaves the View's hints on the status line.
    fn hints(_view: &ToastHistoryViewModel) -> Vec<Hint> {
        Vec::new()
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
