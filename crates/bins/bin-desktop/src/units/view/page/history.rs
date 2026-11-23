//! PRICE HISTORY (#1386), drawn from the page view model's [`PriceHistory`]: at Medium and Large a
//! section under PRICE SOURCES, WEEK · OPEN · HIGH · LOW · CLOSE · CHANGE scrolling within 10 rows
//! at Medium and 4 at Large, where the two tables already fill the window; at X-Large the docked
//! pane's group, a sparkline over WEEK · CLOSE · CHANGE. Read-only: nothing in it takes a click or
//! a key.

use gpui::{AnyElement, App, PathBuilder, Pixels, Rgba, canvas, div, point, prelude::*, px};
use lib_units::presenter::{HistoryRow, PriceHistory, Trend};

use crate::shell::view::window_size::WindowSize;
use crate::theme::color;

/// Each row's height, so the scrolling body shows a whole number of rows.
const ROW_HEIGHT: Pixels = px(32.0);
/// A price column, right-aligned.
const PRICE_WIDTH: Pixels = px(110.0);
/// The pane's narrower CLOSE and CHANGE columns.
const PANE_PRICE_WIDTH: Pixels = px(110.0);
const SPARKLINE_HEIGHT: Pixels = px(48.0);

/// The rows the section shows before it scrolls.
fn visible_rows(window_size: WindowSize) -> usize {
    match window_size {
        WindowSize::Medium => 10,
        WindowSize::Large | WindowSize::XLarge => 4,
    }
}

/// The change's colour: green for a rise, red for a fall, which its sign also says.
fn trend_colour(trend: Trend, cx: &App) -> Option<Rgba> {
    match trend {
        Trend::Rising => Some(color::positive_text(cx)),
        Trend::Falling => Some(color::negative_text(cx)),
        Trend::Flat => None,
    }
}

/// The section under PRICE SOURCES, at Medium and Large.
pub(super) fn section(history: &PriceHistory, window_size: WindowSize, cx: &App) -> AnyElement {
    let muted = color::muted(cx);
    let title = div()
        .flex()
        .items_center()
        .gap(px(8.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .text_color(color::faint_text(cx))
                .child(lib_locale::format::upper(&history.heading)),
        )
        .child(
            div()
                .debug_selector(|| "units-history-subtitle".to_string())
                .text_size(px(11.5))
                .text_color(muted)
                .child(history.subtitle.clone()),
        )
        .children(
            history
                .flag
                .clone()
                .map(|flag| tag(flag, "units-history-flag", cx)),
        );
    let heading = div()
        .flex()
        .items_center()
        .justify_between()
        .pb(px(5.0))
        .mb(px(8.0))
        .border_b(px(2.0))
        .border_color(color::foreground(cx))
        .child(title)
        .child(
            div()
                .text_size(px(11.5))
                .text_color(muted)
                .child(history.open_hint.clone()),
        );
    let body = match &history.empty {
        Some(empty) => empty_line(empty.clone(), cx),
        None => table(history, visible_rows(window_size), cx),
    };
    div()
        .debug_selector(|| "units-history".to_string())
        .mt(px(32.0))
        .flex()
        .flex_col()
        .child(heading)
        .child(body)
        .children(history.count.clone().map(|count| hint(count, cx)))
        .into_any_element()
}

/// The pane's PRICE HISTORY body, at X-Large: the sparkline, then WEEK · CLOSE · CHANGE.
pub(super) fn pane_body(history: &PriceHistory, cx: &App) -> AnyElement {
    if let Some(empty) = &history.empty {
        return empty_line(empty.clone(), cx);
    }
    let columns = &history.columns;
    let last = history.rows.len().saturating_sub(1);
    div()
        .flex()
        .flex_col()
        .child(
            div()
                .debug_selector(|| "units-history-sparkline".to_string())
                .mb(px(6.0))
                .child(sparkline(history.sparkline.clone(), color::foreground(cx))),
        )
        .child(
            pane_cells([&columns.week, &columns.close, &columns.change])
                .py(px(5.0))
                .border_b(px(1.0))
                .border_color(color::hairline(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .text_color(color::muted(cx)),
        )
        .children(history.rows.iter().enumerate().map(|(index, row)| {
            let change_colour = trend_colour(row.trend, cx);
            div()
                .debug_selector(move || format!("units-history-pane-row-{index}"))
                .flex()
                .items_center()
                .py(px(4.0))
                .text_size(px(12.0))
                .when(index != last, |this| {
                    this.border_b(px(1.0)).border_color(color::hairline(cx))
                })
                .child(week_cell(row, cx).flex_1())
                .child(
                    price_cell(row.close.clone(), PANE_PRICE_WIDTH)
                        .font_weight(gpui::FontWeight::EXTRA_BOLD),
                )
                .child(
                    price_cell(row.change.clone(), PANE_PRICE_WIDTH)
                        .when_some(change_colour, |this, colour| this.text_color(colour)),
                )
        }))
        .children(history.pane_hint.clone().map(|text| hint(text, cx)))
        .into_any_element()
}

/// The section's table: a header row over the weeks, scrolling within `visible` rows.
fn table(history: &PriceHistory, visible: usize, cx: &App) -> AnyElement {
    let columns = &history.columns;
    let shown = history.rows.len().min(visible);
    let last = history.rows.len().saturating_sub(1);
    div()
        .debug_selector(|| "units-history-table".to_string())
        .flex()
        .flex_col()
        .border_1()
        .border_color(color::border(cx))
        .child(
            div()
                .flex()
                .items_center()
                .px(px(16.0))
                .py(px(10.0))
                .bg(color::chrome(cx))
                .border_b(px(1.0))
                .border_color(color::border(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .text_color(color::muted(cx))
                .child(
                    div()
                        .flex_1()
                        .child(lib_locale::format::upper(&columns.week)),
                )
                .children(
                    [
                        &columns.open,
                        &columns.high,
                        &columns.low,
                        &columns.close,
                        &columns.change,
                    ]
                    .map(|text| price_cell(lib_locale::format::upper(text), PRICE_WIDTH)),
                ),
        )
        .child(
            div()
                .debug_selector(|| "units-history-rows".to_string())
                .id("units-history-rows")
                .h(ROW_HEIGHT * shown as f32)
                .overflow_y_scroll()
                .flex()
                .flex_col()
                .children(
                    history
                        .rows
                        .iter()
                        .enumerate()
                        .map(|(index, row)| table_row(row, index, index == last, cx)),
                ),
        )
        .into_any_element()
}

fn table_row(row: &HistoryRow, index: usize, last: bool, cx: &App) -> impl IntoElement {
    let muted = color::muted(cx);
    let change_colour = trend_colour(row.trend, cx);
    div()
        .debug_selector(move || format!("units-history-row-{index}"))
        .flex_none()
        .h(ROW_HEIGHT)
        .flex()
        .items_center()
        .px(px(16.0))
        .text_size(px(12.5))
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::hairline(cx))
        })
        .child(week_cell(row, cx).flex_1())
        .children(
            [&row.open, &row.high, &row.low]
                .map(|text| price_cell(text.clone(), PRICE_WIDTH).text_color(muted)),
        )
        .child(price_cell(row.close.clone(), PRICE_WIDTH).font_weight(gpui::FontWeight::EXTRA_BOLD))
        .child(
            price_cell(row.change.clone(), PRICE_WIDTH)
                .when_some(change_colour, |this, colour| this.text_color(colour)),
        )
}

/// The week's Monday, and the current week's `provisional` tag.
fn week_cell(row: &HistoryRow, cx: &App) -> gpui::Div {
    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .child(row.label.clone())
        .children(
            row.provisional
                .clone()
                .map(|text| tag(text, "units-history-provisional", cx)),
        )
}

fn pane_cells(texts: [&String; 3]) -> gpui::Div {
    let [week, close, change] = texts;
    div()
        .flex()
        .items_center()
        .child(div().flex_1().child(lib_locale::format::upper(week)))
        .child(price_cell(
            lib_locale::format::upper(close),
            PANE_PRICE_WIDTH,
        ))
        .child(price_cell(
            lib_locale::format::upper(change),
            PANE_PRICE_WIDTH,
        ))
}

fn price_cell(text: String, width: Pixels) -> gpui::Div {
    div().w(width).flex_none().flex().justify_end().child(text)
}

/// A small bordered muted tag: `provisional`, `this week · provisional`.
fn tag(text: String, selector: &'static str, cx: &App) -> impl IntoElement {
    div()
        .debug_selector(move || selector.to_string())
        .flex_none()
        .px(px(4.0))
        .border_1()
        .border_color(color::border(cx))
        .text_size(px(10.0))
        .font_weight(gpui::FontWeight::BOLD)
        .text_color(color::muted(cx))
        .child(text)
}

fn hint(text: String, cx: &App) -> impl IntoElement {
    div()
        .mt(px(6.0))
        .text_size(px(11.5))
        .text_color(color::muted(cx))
        .child(text)
}

fn empty_line(text: String, cx: &App) -> AnyElement {
    div()
        .debug_selector(|| "units-history-empty".to_string())
        .py(px(16.0))
        .px(px(8.0))
        .text_size(px(12.0))
        .text_color(color::faint_text(cx))
        .child(text)
        .into_any_element()
}

/// The closes, oldest first, as one stroked line filling the width.
fn sparkline(closes: Vec<f64>, stroke: Rgba) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, (), window, _| {
            let (min, max) = closes
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), close| {
                    (min.min(*close), max.max(*close))
                });
            let span = if max > min { max - min } else { 1.0 };
            let steps = closes.len().saturating_sub(1).max(1) as f32;
            let width = f32::from(bounds.size.width) - 6.0;
            let height = f32::from(bounds.size.height) - 8.0;
            let mut builder = PathBuilder::stroke(px(1.6));
            for (index, close) in closes.iter().enumerate() {
                let x = 3.0 + width * index as f32 / steps;
                let y = 4.0 + height * (1.0 - ((close - min) / span) as f32);
                let at = bounds.origin + point(px(x), px(y));
                if index == 0 {
                    builder.move_to(at);
                } else {
                    builder.line_to(at);
                }
            }
            if let Ok(path) = builder.build() {
                window.paint_path(path, stroke);
            }
        },
    )
    .w_full()
    .h(SPARKLINE_HEIGHT)
}
