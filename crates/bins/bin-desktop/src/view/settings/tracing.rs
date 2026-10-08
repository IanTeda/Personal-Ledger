//! The **Tracing (Logs)** page (16m, issues #499–#502): level radios on the left and **Clear logs**
//! on the right of one row, then a log box filling the rest of the page with the live capture,
//! newest first. The box is a virtualised `list` (rows wrap, so heights vary) with an
//! always-visible scroll bar; `Shell` owns its `ListState` and splices it as entries arrive.
//!
//! One entry is `[hh:mm:ss] LEVEL subsystem: message key=value` (`crate::settings::tracing_log`): the time is
//! a fixed column, and a long body wraps under the level tag, not under the time.
//!
//! Element ids are namespaced `tracing-level-*`/`settings-clear-logs`
//! (`docs/ux/desktop-mockups/16-settings/README.md`'s implementation note 11: "namespace radio groups per
//! instance").

use std::{rc::Rc, sync::Arc};

use gpui::{
    AnyElement, App, HighlightStyle, ListState, Rgba, SharedString, StyledText, Window, div, list,
    prelude::*, px,
};
use gpui_component::scroll::{Scrollbar, ScrollbarShow};
use lib_tracing::LogEntry;
use tracing::Level;

use crate::{
    settings::tracing_log::{self, TracingLevel},
    theme::color,
};

pub type OnLevelClick = Rc<dyn Fn(TracingLevel, &mut Window, &mut App)>;
pub type OnClearLogsClick = Rc<dyn Fn(&mut Window, &mut App)>;
/// A plain, level-less click handler -- what a [`radio_option`] is bound to after its own level
/// has already been curried in (mirrors `units::OnPlainClick`).
type OnPlainClick = Rc<dyn Fn(&mut Window, &mut App)>;

const DOT_SIZE: gpui::Pixels = px(16.0);
const DOT_INNER_SIZE: gpui::Pixels = px(8.0);
const DOT_BORDER: gpui::Pixels = px(1.5);
const LOG_TEXT_SIZE: gpui::Pixels = px(11.0);

/// What the log box shows, gathered by `Shell` from its `LogView`.
pub struct LogBoxProps {
    pub level: TracingLevel,
    /// Filtered, newest first.
    pub entries: Rc<Vec<Arc<LogEntry>>>,
    /// Captured entries the filter hides, for the empty state.
    pub hidden: usize,
    pub list: ListState,
}

pub fn render(
    log: &LogBoxProps,
    on_level_click: OnLevelClick,
    on_clear_logs_click: OnClearLogsClick,
    cx: &App,
) -> AnyElement {
    div()
        .flex_1()
        .min_h(px(0.0))
        .flex()
        .flex_col()
        .child(
            div()
                .flex_none()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(12.0))
                .mb(px(10.0))
                .child(level_row(log.level, on_level_click, cx))
                .child(clear_logs_button(on_clear_logs_click, cx)),
        )
        .child(log_box(log, cx))
        .into_any_element()
}

fn level_row(selected: TracingLevel, on_click: OnLevelClick, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .gap(px(8.0))
        .children(TracingLevel::ALL.into_iter().map(|level| {
            let on_click = on_click.clone();
            radio_option(
                level,
                level == selected,
                Rc::new(move |window: &mut Window, cx: &mut App| on_click(level, window, cx)),
                cx,
            )
        }))
}

fn radio_option(
    level: TracingLevel,
    checked: bool,
    on_click: OnPlainClick,
    cx: &App,
) -> impl IntoElement {
    let id = format!("tracing-level-{}", format!("{level:?}").to_lowercase());
    div()
        .debug_selector({
            let id = id.clone();
            move || id.clone()
        })
        .id(SharedString::from(id))
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(8.0))
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(radio_dot(checked, cx))
        .child(div().text_size(px(12.0)).child(level.label()))
}

/// The `.dot` control: a 16px circle, `border:1.5px solid` (`divider`, `accent` when checked),
/// with a centred 8px accent-filled inner circle standing in for the mockup's own `box-shadow:
/// inset 0 0 0 4px var(--color-bg)` ring effect -- `gpui` has no inset-shadow primitive, so two
/// concentric circles reproduce the same "accent ring around a punched-out centre" look.
/// `pub(super)`: reused by `super::display`'s own "Status glyphs" radio group (issue #179) --
/// its second consumer, promoted the same way `add_unit_dialog::segmented_control` was.
pub(super) fn radio_dot(checked: bool, cx: &App) -> impl IntoElement {
    div()
        .w(DOT_SIZE)
        .h(DOT_SIZE)
        .rounded_full()
        .border(DOT_BORDER)
        .border_color(if checked {
            color::accent(cx)
        } else {
            color::divider(cx)
        })
        .bg(color::background(cx))
        .flex()
        .items_center()
        .justify_center()
        .when(checked, |this| {
            this.child(
                div()
                    .w(DOT_INNER_SIZE)
                    .h(DOT_INNER_SIZE)
                    .rounded_full()
                    .bg(color::accent(cx)),
            )
        })
}

/// The log box: `flex:1; min-height:0; border:1px solid rgba(32,30,29,.30); #eae9e9;
/// padding:10px 12px`, monospace 11px/1.6 muted, with the scroll bar always shown.
fn log_box(log: &LogBoxProps, cx: &App) -> impl IntoElement {
    let frame = div()
        .id("tracing-log-box")
        .debug_selector(|| "tracing-log-box".to_string())
        .relative()
        .flex_1()
        .min_h(px(0.0))
        .border_1()
        .border_color(color::border(cx))
        .bg(color::chrome(cx))
        .font_family("monospace")
        .text_size(LOG_TEXT_SIZE)
        .line_height(gpui::relative(1.6))
        .text_color(color::muted(cx));

    if log.entries.is_empty() {
        let message = if log.hidden == 0 {
            crate::msg::desktop_settings_tracing_empty()
        } else {
            crate::msg::desktop_settings_tracing_all_hidden(
                &log.level.label(),
                i64::try_from(log.hidden).unwrap_or(i64::MAX),
            )
        };
        return frame
            .flex()
            .items_center()
            .justify_center()
            .p(px(24.0))
            .child(
                div()
                    .debug_selector(|| "tracing-log-empty".to_string())
                    .text_color(color::faint_text(cx))
                    .font_family(crate::theme::type_scale::FONT_FAMILY)
                    .child(message),
            );
    }

    let entries = log.entries.clone();
    let rows = list(log.list.clone(), move |index, _window, cx| {
        match entries.get(index) {
            Some(entry) => entry_row(index, entry, cx),
            // The list's count can briefly lead the entries during a splice; draw nothing.
            None => div().into_any_element(),
        }
    })
    .size_full()
    .py(px(10.0))
    .px(px(12.0));

    frame.child(rows).child(
        div()
            .occlude()
            .absolute()
            .top_0()
            .right_0()
            .bottom_0()
            .w(px(12.0))
            .child(Scrollbar::vertical(&log.list).scrollbar_show(ScrollbarShow::Always)),
    )
}

/// One entry: the time in a fixed column, then the level tag (coloured) and body as one wrapping
/// run, so continuation lines hang under the level tag.
fn entry_row(index: usize, entry: &LogEntry, cx: &App) -> AnyElement {
    let tag = tracing_log::level_tag(entry.level);
    let text = format!("{tag} {}", tracing_log::body(entry));
    let highlight = HighlightStyle {
        color: Some(level_colour(entry.level, cx).into()),
        font_weight: (entry.level <= Level::WARN).then_some(gpui::FontWeight::EXTRA_BOLD),
        ..Default::default()
    };
    div()
        .debug_selector(move || format!("tracing-log-row-{index}"))
        .flex()
        .gap(px(8.0))
        .pr(px(12.0))
        .child(
            div()
                .flex_none()
                .text_color(color::faint_text(cx))
                .child(tracing_log::timestamp(entry.time)),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .child(StyledText::new(text).with_highlights([(0..tag.len(), highlight)])),
        )
        .into_any_element()
}

/// Error and warning use the Toast marks, which hold 3:1 against the box's chrome ground; info
/// stays muted like the body and debug drops back to faint.
fn level_colour(level: Level, cx: &App) -> Rgba {
    match level {
        Level::ERROR => color::toast_mark(lib_toast::ToastKind::Error, cx),
        Level::WARN => color::toast_mark(lib_toast::ToastKind::Warning, cx),
        Level::INFO => color::muted(cx),
        Level::DEBUG | Level::TRACE => color::faint_text(cx),
    }
}

/// **Clear logs**: `.btn.btn-secondary`, 32px, top right of the level row.
fn clear_logs_button(on_click: OnClearLogsClick, cx: &App) -> impl IntoElement {
    div()
        .id("settings-clear-logs")
        .debug_selector(|| "settings-clear-logs".to_string())
        .cursor_pointer()
        .flex_none()
        .h(px(32.0))
        .flex()
        .items_center()
        .px(px(16.0))
        .bg(color::chrome(cx))
        .border_1()
        .border_color(color::border(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .whitespace_nowrap()
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(crate::msg::desktop_settings_tracing_clear())
}
