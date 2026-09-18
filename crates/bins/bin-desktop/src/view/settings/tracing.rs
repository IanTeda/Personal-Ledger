//! The **Tracing (Logs)** section (`docs/ux/desktop/Settings/README.md`'s "2a resting state"): a
//! 400px column of level radios (error/warn/info/debug), a scrolling monospace log viewport, and
//! a **Clear logs** button. Unlike every other button this map has built so far, "Clear logs" has
//! a real effect: it empties `Shell`-owned `settings_log_lines`, since the ticket's own body asks
//! for that specifically (not a permanently-out-of-scope stand-in like Sync server's/Data &
//! backup's buttons).
//!
//! The level radios are a dot-style `.radio`/`.dot` control (`docs/ux/desktop/Shell &
//! Navigation/styles.css`), not the segmented-box `.seg`/`.seg-opt` style `ledger_units` uses --
//! the first use of this component in the crate. There are no real log lines to filter by level
//! yet, so selecting one is a stored preference only, same as `ledger_units`'s own controls
//! before any downstream effect existed.
//!
//! Element ids are namespaced `tracing-level-*`/`settings-clear-logs`
//! (`docs/ux/desktop/Settings/README.md`'s implementation note 11: "namespace radio groups per
//! instance" so a duplicated section's own radios can't collide with these).

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};

use crate::{settings::TracingLevel, theme::color};

pub type OnLevelClick = Rc<dyn Fn(TracingLevel, &mut Window, &mut App)>;
pub type OnClearLogsClick = Rc<dyn Fn(&mut Window, &mut App)>;
/// A plain, level-less click handler -- what a [`radio_option`] is bound to after its own level
/// has already been curried in (mirrors `units::OnPlainClick`).
type OnPlainClick = Rc<dyn Fn(&mut Window, &mut App)>;

const COLUMN_WIDTH: gpui::Pixels = px(400.0);
const VIEWPORT_HEIGHT: gpui::Pixels = px(120.0);
const DOT_SIZE: gpui::Pixels = px(16.0);
const DOT_INNER_SIZE: gpui::Pixels = px(8.0);
const DOT_BORDER: gpui::Pixels = px(1.5);

pub fn render(
    selected: TracingLevel,
    log_lines: &[&'static str],
    on_level_click: OnLevelClick,
    on_clear_logs_click: OnClearLogsClick,
) -> AnyElement {
    div()
        .w(COLUMN_WIDTH)
        .flex_none()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .child(level_row(selected, on_level_click))
        .child(log_viewport(log_lines))
        .child(clear_logs_button(on_clear_logs_click))
        .into_any_element()
}

fn level_row(selected: TracingLevel, on_click: OnLevelClick) -> impl IntoElement {
    div()
        .flex()
        .gap(px(8.0))
        .mb(px(8.0))
        .children(TracingLevel::ALL.into_iter().map(|level| {
            let on_click = on_click.clone();
            radio_option(
                level,
                level == selected,
                Rc::new(move |window: &mut Window, cx: &mut App| on_click(level, window, cx)),
            )
        }))
}

fn radio_option(level: TracingLevel, checked: bool, on_click: OnPlainClick) -> impl IntoElement {
    div()
        .id(SharedString::from(format!(
            "tracing-level-{}",
            level.label()
        )))
        .cursor_pointer()
        .flex()
        .items_center()
        .gap(px(8.0))
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(radio_dot(checked))
        .child(div().text_size(px(12.0)).child(level.label()))
}

/// The `.dot` control: a 16px circle, `border:1.5px solid` (`DIVIDER`, `ACCENT` when checked),
/// with a centred 8px accent-filled inner circle standing in for the mockup's own `box-shadow:
/// inset 0 0 0 4px var(--color-bg)` ring effect -- `gpui` has no inset-shadow primitive, so two
/// concentric circles reproduce the same "accent ring around a punched-out centre" look.
fn radio_dot(checked: bool) -> impl IntoElement {
    div()
        .w(DOT_SIZE)
        .h(DOT_SIZE)
        .rounded_full()
        .border(DOT_BORDER)
        .border_color(if checked {
            color::ACCENT
        } else {
            color::DIVIDER
        })
        .bg(color::GROUND)
        .flex()
        .items_center()
        .justify_center()
        .when(checked, |this| {
            this.child(
                div()
                    .w(DOT_INNER_SIZE)
                    .h(DOT_INNER_SIZE)
                    .rounded_full()
                    .bg(color::ACCENT),
            )
        })
}

/// The log viewport: `border:1px solid rgba(32,30,29,.30); background:#eae9e9; padding:10px;
/// height:120px; overflow-y:auto; font-family:monospace; font-size:10px; line-height:1.5;
/// color:#605d5d`.
fn log_viewport(log_lines: &[&'static str]) -> impl IntoElement {
    div()
        .id("tracing-log-viewport")
        .h(VIEWPORT_HEIGHT)
        .border_1()
        .border_color(color::BORDER)
        .bg(color::CHROME)
        .p(px(10.0))
        .overflow_y_scroll()
        .font_family("monospace")
        .text_size(px(10.0))
        .line_height(gpui::relative(1.5))
        .text_color(color::INK_SECONDARY)
        .flex()
        .flex_col()
        .children(
            log_lines
                .iter()
                .enumerate()
                .map(|(index, line)| div().id(("tracing-log-line", index)).child(*line)),
        )
}

/// The "Clear logs" button: `padding:8px 16px; border:1px solid rgba(32,30,29,.30);
/// background:#eae9e9; font-weight:800; margin-top:8px` -- same shape as
/// `sync_server::sync_now_button`, but with a real effect on click.
fn clear_logs_button(on_click: OnClearLogsClick) -> impl IntoElement {
    let mut button = div()
        .id("settings-clear-logs")
        .cursor_pointer()
        .mt(px(8.0))
        .py(px(8.0))
        .px(px(16.0))
        .bg(color::CHROME)
        .border_1()
        .border_color(color::BORDER)
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .whitespace_nowrap()
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child("Clear logs");
    button.style().align_self = Some(gpui::AlignItems::FlexStart);
    button
}
