//! The **9c** History tab (`docs/ux/desktop/design_handoff_budgets_v2/limits-9a-9g.md`): a chart with one pair
//! of bars per month (effective budget and Spent over the budgeted leaves), then a table of Spent
//! per Category and month with its AVG and OVER columns.
//!
//! An over-budget month or cell takes the accent and a `▲`; the current month is drawn outlined
//! and labelled "to date", and counts in neither AVG nor OVER. A month in which a row was
//! unbudgeted shows `—`. The cursor cell inverts. Every input is a callback into `Shell`.

use std::rc::Rc;

use bigdecimal::{BigDecimal, ToPrimitive, Zero};
use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};
use lib_core::Money;
use lib_locale::format::{format_month, upper};

use crate::{
    budgets::{History, HistoryCell, HistoryMonth, HistoryRow},
    categories::{self, Category},
    format::amount,
    theme::color,
    transaction_rows::EMPTY_CELL,
};

const BUDGET_WIDTH: gpui::Pixels = px(92.0);
const MONTH_WIDTH: gpui::Pixels = px(92.0);
const AVERAGE_WIDTH: gpui::Pixels = px(92.0);
const OVER_WIDTH: gpui::Pixels = px(56.0);
const CHART_HEIGHT: f32 = 120.0;
const INDENT: f32 = 16.0;

/// Called with a cell's row and month column.
pub type OnCellClick = Rc<dyn Fn(usize, usize, &mut Window, &mut App)>;

pub struct HistoryProps<'a> {
    pub history: &'a History,
    pub categories: &'a [Category],
    /// The cursor's row and month column.
    pub cursor: (usize, usize),
    pub on_cell_click: OnCellClick,
}

fn text(money: &Money) -> String {
    amount(money).1
}

/// `Apr – Sep 2026`, or with both years when the range crosses one.
pub fn range_label(history: &History) -> String {
    let (Some(first), Some(last)) = (history.months.first(), history.months.last()) else {
        return String::new();
    };
    let (first, last) = (first.month, last.month);
    if first.year == last.year {
        crate::msg::desktop_budgets_plan_range(
            &format_month(first.month),
            &format_month(last.month),
            &last.year.to_string(),
        )
    } else {
        crate::msg::desktop_budgets_plan_range_years(
            &format_month(first.month),
            &first.year.to_string(),
            &format_month(last.month),
            &last.year.to_string(),
        )
    }
}

pub fn render(props: &HistoryProps<'_>, cx: &App) -> AnyElement {
    if props.history.rows.is_empty() {
        return div()
            .text_color(color::muted(cx))
            .child(crate::msg::desktop_budgets_history_empty())
            .into_any_element();
    }
    div()
        .flex()
        .flex_col()
        .gap(px(24.0))
        .child(chart(props.history, cx))
        .child(table(props, cx))
        .into_any_element()
}

/// One column per month: the budget and Spent bars scaled against the range's largest figure,
/// the pair's figures and the month under them, then the closed-months caption.
fn chart(history: &History, cx: &App) -> impl IntoElement {
    let tallest = history
        .months
        .iter()
        .flat_map(|month| [&month.budget.0, &month.spent.0])
        .max()
        .cloned()
        .filter(|value| *value > BigDecimal::zero());
    let height = |value: &Money| {
        let share = tallest.as_ref().map_or(0.0, |tallest| {
            (value.0.clone() / tallest.clone())
                .to_f32()
                .unwrap_or(0.0)
                .clamp(0.0, 1.0)
        });
        px(CHART_HEIGHT * share)
    };
    let closed: Vec<&HistoryMonth> = history.months.iter().filter(|m| !m.is_current).collect();
    let caption = if closed.is_empty() {
        crate::msg::desktop_budgets_history_caption_none()
    } else {
        crate::msg::desktop_budgets_history_caption(
            i64::try_from(closed.len()).unwrap_or(i64::MAX),
            &closed.iter().filter(|m| m.over).count().to_string(),
        )
    };
    div()
        .flex()
        .flex_col()
        .gap(px(10.0))
        .child(
            div()
                .flex()
                .gap(px(16.0))
                .children(history.months.iter().map(|month| {
                    let ink = color::foreground(cx);
                    let spent_fill = if month.over { color::negative(cx) } else { ink };
                    let mut spent_bar = div().w(px(22.0)).h(height(&month.spent));
                    // "To date" is still moving, so it is outlined rather than solid.
                    spent_bar = if month.is_current {
                        spent_bar.border_1().border_color(spent_fill)
                    } else {
                        spent_bar.bg(spent_fill)
                    };
                    let mut label = format_month(month.month.month);
                    if month.is_current {
                        label = format!(
                            "{label} \u{b7} {}",
                            crate::msg::desktop_budgets_history_to_date()
                        );
                    } else if month.over {
                        label = format!(
                            "{label} \u{b7} {}",
                            crate::msg::desktop_budgets_history_over_mark()
                        );
                    }
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
                                        .h(height(&month.budget))
                                        .bg(color::inset_track(cx)),
                                )
                                .child(spent_bar),
                        )
                        .child(
                            div()
                                .text_size(px(11.0))
                                .whitespace_nowrap()
                                .when(month.over, |this| this.text_color(color::accent_text(cx)))
                                .child(crate::msg::desktop_budgets_history_pair(
                                    &text(&month.spent),
                                    &text(&month.budget),
                                )),
                        )
                        .child(
                            div()
                                .text_size(px(10.0))
                                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                                .text_color(color::muted(cx))
                                .child(upper(&label)),
                        )
                })),
        )
        .child(
            div()
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .child(caption),
        )
}

fn table(props: &HistoryProps<'_>, cx: &App) -> impl IntoElement {
    let count = props.history.rows.len();
    div()
        .flex()
        .flex_col()
        .border_1()
        .border_color(color::border(cx))
        .child(table_header(props.history, cx))
        .children(
            props
                .history
                .rows
                .iter()
                .enumerate()
                .map(|(index, row)| render_row(index, row, index + 1 == count, props, cx)),
        )
}

fn table_header(history: &History, cx: &App) -> impl IntoElement {
    let cell = |width, label: String| {
        div()
            .w(width)
            .flex_none()
            .px(px(6.0))
            .text_align(gpui::TextAlign::Right)
            .whitespace_nowrap()
            .child(upper(&label))
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
                .child(upper(&lib_locale::msg::column_category())),
        )
        .child(cell(
            BUDGET_WIDTH,
            crate::msg::desktop_budgets_column_budget(),
        ))
        .children(
            history
                .months
                .iter()
                .map(|month| cell(MONTH_WIDTH, format_month(month.month.month))),
        )
        .child(cell(
            AVERAGE_WIDTH,
            crate::msg::desktop_budgets_history_column_avg(),
        ))
        .child(cell(
            OVER_WIDTH,
            crate::msg::desktop_budgets_history_column_over(),
        ))
}

/// The row's BUDGET column: the effective budget of the latest month shown in which it had one.
fn latest_budget(row: &HistoryRow) -> Option<&Money> {
    row.cells
        .iter()
        .rev()
        .find_map(|cell| cell.as_ref().map(|cell| &cell.budget))
}

fn render_row(
    index: usize,
    row: &HistoryRow,
    last: bool,
    props: &HistoryProps<'_>,
    cx: &App,
) -> AnyElement {
    let name = props
        .categories
        .iter()
        .find(|c| c.id == row.category_id)
        .map_or_else(|| EMPTY_CELL.to_string(), |c| c.name.clone());
    let depth = categories::depth(props.categories, row.category_id) as f32;
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
                .pl(px(depth * INDENT))
                .when(row.is_parent || props.cursor.0 == index, |this| {
                    this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                })
                .child(if row.is_parent {
                    format!("\u{25be} {name}")
                } else {
                    name
                }),
        )
        .child(
            figure(
                BUDGET_WIDTH,
                latest_budget(row).map_or_else(|| EMPTY_CELL.to_string(), text),
            )
            .text_color(color::muted(cx)),
        )
        .children(
            row.cells
                .iter()
                .enumerate()
                .map(|(column, cell)| render_cell(index, column, cell.as_ref(), props, cx)),
        )
        .child(figure(
            AVERAGE_WIDTH,
            row.average
                .as_ref()
                .map_or_else(|| EMPTY_CELL.to_string(), text),
        ))
        .child(
            figure(
                OVER_WIDTH,
                if row.closed_months == 0 {
                    EMPTY_CELL.to_string()
                } else {
                    crate::msg::desktop_budgets_history_over_count(
                        &row.over_months.to_string(),
                        &row.closed_months.to_string(),
                    )
                },
            )
            .when(row.over_months > 0, |this| {
                this.text_color(color::accent_text(cx))
            }),
        )
        .into_any_element()
}

fn render_cell(
    index: usize,
    column: usize,
    cell: Option<&HistoryCell>,
    props: &HistoryProps<'_>,
    cx: &App,
) -> AnyElement {
    let on_click = props.on_cell_click.clone();
    let cursor_here = props.cursor == (index, column);
    let over = cell.is_some_and(|cell| cell.over);
    let value = cell.map_or_else(
        || EMPTY_CELL.to_string(),
        |cell| {
            if cell.over {
                format!("\u{25b2} {}", text(&cell.spent))
            } else {
                text(&cell.spent)
            }
        },
    );
    div()
        .id(SharedString::from(format!(
            "budgets-history-{index}-{column}"
        )))
        .w(MONTH_WIDTH)
        .flex_none()
        .py(px(4.0))
        .px(px(6.0))
        .text_align(gpui::TextAlign::Right)
        .whitespace_nowrap()
        .cursor_pointer()
        .when(cell.is_none(), |this| this.text_color(color::muted(cx)))
        .when(over && !cursor_here, |this| {
            this.text_color(color::accent_text(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
        })
        .when(cursor_here, |this| {
            this.bg(color::selection_background(cx))
                .text_color(if over {
                    color::selection_accent_text(cx)
                } else {
                    color::selection_text(cx)
                })
        })
        .on_click(move |_event, window, cx| on_click(index, column, window, cx))
        .child(value)
        .into_any_element()
}
