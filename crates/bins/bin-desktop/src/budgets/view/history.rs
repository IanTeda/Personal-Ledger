//! The **9c** History tab (`docs/ux/desktop-mockups/14-budgets-v2/README.md`): a chart with one
//! pair of bars per month (effective budget and Spent over the budgeted leaves), then a table of
//! Spent per Category and month with its AVG and OVER columns, drawn from the Presenter's
//! [`HistoryViewModel`].
//!
//! An over-budget month or cell takes the accent; the current month is drawn outlined. A month in
//! which a row was unbudgeted is muted. The cursor cell inverts.

use gpui::{AnyElement, App, Entity, SharedString, div, prelude::*, px};
use lib_budgets::presenter::{
    BudgetsPageEvent, HistoryCellView, HistoryEvent, HistoryRowView, HistoryViewModel,
};
use lib_locale::format::upper;

use super::on_event;
use super::page::BudgetsPage;
use crate::theme::color;

const BUDGET_WIDTH: gpui::Pixels = px(92.0);
const MONTH_WIDTH: gpui::Pixels = px(92.0);
const AVERAGE_WIDTH: gpui::Pixels = px(92.0);
const OVER_WIDTH: gpui::Pixels = px(56.0);
const CHART_HEIGHT: f32 = 120.0;
const INDENT: f32 = 16.0;

pub(super) fn render(
    view: &HistoryViewModel,
    entity: &Entity<BudgetsPage>,
    cx: &App,
) -> AnyElement {
    if let Some(empty) = &view.empty {
        return div()
            .text_color(color::muted(cx))
            .child(empty.clone())
            .into_any_element();
    }
    div()
        .flex()
        .flex_col()
        .gap(px(24.0))
        .child(chart(view, cx))
        .child(table(view, entity, cx))
        .into_any_element()
}

/// One column per month: the budget and Spent bars, the pair's figures and the month under them,
/// then the closed-months caption.
fn chart(view: &HistoryViewModel, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(
            div()
                .flex()
                .gap(px(16.0))
                .children(view.bars.iter().map(|bar| {
                    let ink = color::foreground(cx);
                    let spent_fill = if bar.over { color::negative(cx) } else { ink };
                    let spent_bar = div().w(px(22.0)).h(px(CHART_HEIGHT * bar.spent));
                    // "To date" is still moving, so it is outlined rather than solid.
                    let spent_bar = if bar.current {
                        spent_bar.border_1().border_color(spent_fill)
                    } else {
                        spent_bar.bg(spent_fill)
                    };
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex()
                        .flex_col()
                        .gap(px(6.0))
                        .child(
                            div()
                                .h(px(CHART_HEIGHT))
                                .flex()
                                .items_end()
                                .gap(px(4.0))
                                .border_b(px(1.0))
                                .border_color(color::border(cx))
                                .child(
                                    div()
                                        .w(px(22.0))
                                        .h(px(CHART_HEIGHT * bar.budget))
                                        .bg(color::inset_track(cx)),
                                )
                                .child(spent_bar),
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .whitespace_nowrap()
                                .when(bar.over, |this| this.text_color(color::accent_text(cx)))
                                .child(bar.pair.clone()),
                        )
                        .child(
                            div()
                                .text_size(px(10.0))
                                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                                .text_color(color::muted(cx))
                                .child(upper(&bar.label)),
                        )
                })),
        )
        .child(
            div()
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .child(view.caption.clone()),
        )
}

fn table(view: &HistoryViewModel, entity: &Entity<BudgetsPage>, cx: &App) -> impl IntoElement {
    let count = view.rows.len();
    div()
        .flex()
        .flex_col()
        .border_1()
        .border_color(color::border(cx))
        .child(table_header(view, cx))
        .children(
            view.rows
                .iter()
                .enumerate()
                .map(|(index, row)| render_row(index, row, index + 1 == count, entity, cx)),
        )
}

fn table_header(view: &HistoryViewModel, cx: &App) -> impl IntoElement {
    let columns = &view.columns;
    let cell = |width, label: &str| {
        div()
            .w(width)
            .flex_none()
            .px(px(6.0))
            .text_align(gpui::TextAlign::Right)
            .whitespace_nowrap()
            .child(upper(label))
    };
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
                .min_w(px(140.0))
                .child(upper(&columns.category)),
        )
        .child(cell(BUDGET_WIDTH, &columns.budget))
        .children(columns.months.iter().map(|month| cell(MONTH_WIDTH, month)))
        .child(cell(AVERAGE_WIDTH, &columns.average))
        .child(cell(OVER_WIDTH, &columns.over))
}

fn render_row(
    index: usize,
    row: &HistoryRowView,
    last: bool,
    entity: &Entity<BudgetsPage>,
    cx: &App,
) -> AnyElement {
    let figure = |width, value: String| {
        div()
            .w(width)
            .flex_none()
            .py(px(4.0))
            .px(px(6.0))
            .text_align(gpui::TextAlign::Right)
            .child(value)
    };
    div()
        .flex()
        .items_center()
        .px(px(16.0))
        .py(px(2.0))
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::hairline(cx))
        })
        .child(
            div()
                .flex_1()
                .min_w(px(140.0))
                .pl(px(row.depth as f32 * INDENT))
                .when(row.bold, |this| {
                    this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                })
                .child(row.name.clone()),
        )
        .child(figure(BUDGET_WIDTH, row.budget.clone()).text_color(color::muted(cx)))
        .children(
            row.cells
                .iter()
                .enumerate()
                .map(|(column, cell)| render_cell(index, column, cell, entity, cx)),
        )
        .child(figure(AVERAGE_WIDTH, row.average.clone()))
        .child(
            figure(OVER_WIDTH, row.over_count.clone()).when(row.over_alert, |this| {
                this.text_color(color::accent_text(cx))
            }),
        )
        .into_any_element()
}

fn render_cell(
    index: usize,
    column: usize,
    cell: &HistoryCellView,
    entity: &Entity<BudgetsPage>,
    cx: &App,
) -> AnyElement {
    let on_click = on_event(
        entity,
        BudgetsPageEvent::History(HistoryEvent::ClickCell(index, column)),
    );
    div()
        .id(SharedString::from(format!(
            "budgets-history-{index}-{column}"
        )))
        .debug_selector(move || format!("budgets-history-{index}-{column}"))
        .w(MONTH_WIDTH)
        .flex_none()
        .py(px(4.0))
        .px(px(6.0))
        .text_align(gpui::TextAlign::Right)
        .whitespace_nowrap()
        .cursor_pointer()
        .when(cell.empty, |this| this.text_color(color::muted(cx)))
        .when(cell.over && !cell.cursor, |this| {
            this.text_color(color::accent_text(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
        })
        .when(cell.cursor, |this| {
            this.bg(color::selection_background(cx))
                .text_color(if cell.over {
                    color::selection_accent_text(cx)
                } else {
                    color::selection_text(cx)
                })
        })
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(cell.text.clone())
        .into_any_element()
}
