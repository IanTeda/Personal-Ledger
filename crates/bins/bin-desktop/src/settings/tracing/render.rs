use gpui::{
    AnyElement, App, HighlightStyle, ListState, Rgba, SharedString, StyledText, div, list,
    prelude::*, px,
};
use gpui_component::scroll::{Scrollbar, ScrollbarShow};
use lib_settings::log::{LogLevel, LogLine};
use lib_settings::{TracingEvent, TracingLevel, TracingViewModel};

use crate::{
    dialog::OnClick, form::fields::radio_dot, settings::tracing::TracingPage,
    shell::view::page_clicks::PageClicks, theme::color,
};

const LOG_TEXT_SIZE: gpui::Pixels = px(11.0);

/// The level row and Clear logs over the log box, which shows `list`.
pub(super) fn page(
    view: &TracingViewModel,
    list: ListState,
    clicks: &PageClicks<TracingPage>,
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
                .child(level_row(view, clicks, cx))
                .child(clear_logs_button(
                    view.clear_label.clone(),
                    clicks.click(TracingEvent::ClearLogs),
                    cx,
                )),
        )
        .child(log_box(view, list, cx))
        .into_any_element()
}

fn level_row(
    view: &TracingViewModel,
    clicks: &PageClicks<TracingPage>,
    cx: &App,
) -> impl IntoElement {
    div()
        .flex()
        .gap(px(8.0))
        .children(view.levels.iter().map(|choice| {
            radio_option(
                choice.value,
                choice.label.clone(),
                choice.value == view.level,
                clicks.click(TracingEvent::ChooseLevel(choice.value)),
                cx,
            )
        }))
}

fn radio_option(
    level: TracingLevel,
    label: String,
    checked: bool,
    on_click: OnClick,
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
        .child(div().text_size(px(12.0)).child(label))
}

/// The log box: `flex:1; min-height:0; border:1px solid rgba(32,30,29,.30); #eae9e9;
/// padding:10px 12px`, monospace 11px/1.6 muted, with the scroll bar always shown.
fn log_box(view: &TracingViewModel, list_state: ListState, cx: &App) -> impl IntoElement {
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

    if let Some(message) = view.empty.clone() {
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

    let entries = view.entries.clone();
    let rows = list(list_state.clone(), move |index, _window, cx| {
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
            .child(Scrollbar::vertical(&list_state).scrollbar_show(ScrollbarShow::Always)),
    )
}

/// One entry: the time in a fixed column, then the level tag (coloured) and body as one wrapping
/// run, so continuation lines hang under the level tag.
fn entry_row(index: usize, entry: &LogLine, cx: &App) -> AnyElement {
    let tag = entry.tag();
    let text = format!("{tag} {}", entry.body());
    let highlight = HighlightStyle {
        color: Some(level_colour(entry.level, cx).into()),
        font_weight: (entry.level <= LogLevel::Warn).then_some(gpui::FontWeight::EXTRA_BOLD),
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
                .child(entry.timestamp()),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .child(StyledText::new(text).with_highlights([(0..tag.len(), highlight)])),
        )
        .into_any_element()
}

/// Error and warning use the Toast marks, which hold 3:1 against the box's `chrome` ground; info
/// stays muted like the body and debug drops back to faint.
fn level_colour(level: LogLevel, cx: &App) -> Rgba {
    match level {
        LogLevel::Error => color::toast_mark(lib_toast::ToastKind::Error, cx),
        LogLevel::Warn => color::toast_mark(lib_toast::ToastKind::Warning, cx),
        LogLevel::Info => color::muted(cx),
        LogLevel::Debug | LogLevel::Trace => color::faint_text(cx),
    }
}

/// **Clear logs**: `.btn.btn-secondary`, 32px, top right of the level row.
fn clear_logs_button(label: String, on_click: OnClick, cx: &App) -> impl IntoElement {
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
        .child(label)
}
