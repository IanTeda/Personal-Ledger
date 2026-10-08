//! The session Toast history (`docs/toasts-design.md` "Session Toast history", #339): a modal
//! card on `dialog`'s chrome listing every Toast raised this run, newest first -- Kind glyph in
//! its mark colour, the time last raised, the full untruncated Message and its `×N`. The list
//! scrolls; `Esc` or Close shuts it, and viewing dismisses nothing.

use std::rc::Rc;

use chrono::{DateTime, Local};
use gpui::{AnyElement, App, div, prelude::*, px};
use lib_locale::format::format_time;
use lib_toast::{History, ToastKind};

use crate::{
    dialog,
    theme::{color, type_scale},
};

const WIDTH: gpui::Pixels = px(560.0);
/// Past this the list scrolls rather than the card outgrowing a small window.
const LIST_MAX_HEIGHT: gpui::Pixels = px(420.0);

pub type OnClose = dialog::OnClick;

/// One history row as text, so the ordering and merging are testable without a window.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    kind: ToastKind,
    time: String,
    text: String,
    badge: Option<String>,
}

fn rows(history: &History<DateTime<Local>>) -> Vec<Row> {
    history
        .iter()
        .map(|entry| Row {
            kind: entry.kind(),
            time: format_time(entry.last_raised().time()),
            text: entry.text().to_string(),
            badge: (entry.count() > 1).then(|| format!("×{}", entry.count())),
        })
        .collect()
}

pub fn render(history: &History<DateTime<Local>>, on_close: OnClose, cx: &App) -> AnyElement {
    let rows = rows(history);
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
            .children(rows.into_iter().map(|row| render_row(row, cx)))
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

fn render_row(row: Row, cx: &App) -> AnyElement {
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
                .child(row.time),
        )
        // Wraps rather than truncating: the history is where a long Message is read in full.
        .child(div().flex_1().min_w(px(0.0)).child(row.text))
        .children(
            row.badge
                .map(|badge| div().flex_none().text_color(color::muted(cx)).child(badge)),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use lib_toast::Toasts;

    use super::*;

    fn at(second: u32) -> DateTime<Local> {
        Local
            .with_ymd_and_hms(2026, 9, 28, 15, 4, second)
            .single()
            .expect("test time is unambiguous")
    }

    #[test]
    fn an_empty_history_has_no_rows() {
        let toasts: Toasts<DateTime<Local>> = Toasts::default();
        assert!(rows(toasts.history()).is_empty());
    }

    #[test]
    fn rows_list_newest_first() {
        crate::locale::init_for_tests();
        let mut toasts = Toasts::default();
        toasts.raise(ToastKind::Info, "older", at(1));
        toasts.raise(ToastKind::Error, "newer", at(2));
        let texts: Vec<_> = rows(toasts.history())
            .into_iter()
            .map(|row| (row.kind, row.text))
            .collect();
        assert_eq!(
            texts,
            [
                (ToastKind::Error, "newer".to_string()),
                (ToastKind::Info, "older".to_string())
            ]
        );
    }

    #[test]
    fn a_merged_duplicate_is_one_row_with_its_count_and_last_time() {
        crate::locale::init_for_tests();
        let mut toasts = Toasts::default();
        toasts.raise(ToastKind::Success, "Deleted tag Food", at(1));
        toasts.raise(ToastKind::Success, "Deleted tag Food", at(2));
        let rows = rows(toasts.history());
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].badge.as_deref(), Some("×2"));
        assert_eq!(rows[0].time, format_time(at(2).time()));
    }
}
