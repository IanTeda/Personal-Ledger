//! The **9a** Progress tab (`docs/ux/desktop/14-budgets-v2/README.md`): the header's meta line,
//! the legend, the four-cell stat strip and the Category table, all read from one
//! `budgets::period_figures`.
//!
//! A row's bar is solid for Spent, then a bordered muted segment for Known Costs (the handoff
//! hatches it; GPUI has no pattern fill, and the outline keeps it from resting on colour alone),
//! with a 2px tick at the period's elapsed share. An over-budget bar fills in the negative colour
//! and its LEFT reads `68.40 over`. The selected row inverts. The `edit`/`set` actions and the
//! row's Enter detail belong to #403 and #405.

use bigdecimal::{BigDecimal, Signed, ToPrimitive, Zero};
use gpui::{AnyElement, App, Rgba, SharedString, div, prelude::*, px, relative};
use lib_core::Money;
use lib_locale::format::upper;

use crate::{
    budgets::{CategoryFigures, PeriodFigures},
    categories,
    format::amount,
    theme::color,
    transactions::rows::EMPTY_CELL,
};

use super::{BudgetsPageProps, period_label};

const BUDGET_WIDTH: gpui::Pixels = px(96.0);
const SPENT_WIDTH: gpui::Pixels = px(96.0);
const KNOWN_WIDTH: gpui::Pixels = px(96.0);
const LEFT_WIDTH: gpui::Pixels = px(130.0);
const PROGRESS_WIDTH: gpui::Pixels = px(190.0);
const ACTIONS_WIDTH: gpui::Pixels = px(60.0);
const INDENT: f32 = 18.0;

fn text(money: &Money) -> String {
    amount(money).1
}

/// Whole percent of `part` in `whole`, `0` when there is no whole.
fn percent(part: &Money, whole: &Money) -> i64 {
    if whole.0.is_zero() {
        return 0;
    }
    let ratio: BigDecimal = part.0.clone() * 100 / whole.0.clone();
    ratio.round(0).to_i64().unwrap_or(0)
}

/// `September 2026 · day 21 of 30 · 1 over budget · 58.00 unbudgeted`. The day is the current
/// month's only, and a zero clause is left out.
pub fn meta_line(props: &BudgetsPageProps<'_>, cx: &App) -> impl IntoElement {
    let figures = props.figures;
    let strong = |segments: Vec<lib_locale::Segment>, colour: Rgba| {
        div()
            .flex()
            .children(segments.into_iter().map(move |segment| {
                match segment.tag.as_deref() {
                    Some("strong") => div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_color(colour)
                        .child(segment.text)
                        .into_any_element(),
                    _ => div().child(segment.text).into_any_element(),
                }
            }))
    };
    let mut clauses: Vec<AnyElement> = vec![
        div()
            .child(crate::msg::desktop_budgets_meta_period(&period_label(
                props.period,
            )))
            .into_any_element(),
    ];
    let elapsed = figures.elapsed;
    if elapsed.day > 0 && elapsed.days_left > 0 {
        clauses.push(
            div()
                .child(crate::msg::desktop_budgets_meta_day(
                    &elapsed.day.to_string(),
                    &elapsed.days_in_month.to_string(),
                ))
                .into_any_element(),
        );
    }
    if figures.over_count > 0 {
        clauses.push(
            strong(
                crate::msg::desktop_budgets_meta_over(&figures.over_count.to_string()),
                color::accent_text(cx),
            )
            .into_any_element(),
        );
    }
    if figures.at_risk_count > 0 {
        clauses.push(
            div()
                .child(crate::msg::desktop_budgets_meta_at_risk(
                    &figures.at_risk_count.to_string(),
                ))
                .into_any_element(),
        );
    }
    if figures.unbudgeted_spent.0.is_positive() {
        clauses.push(
            div()
                .child(crate::msg::desktop_budgets_meta_unbudgeted(&text(
                    &figures.unbudgeted_spent,
                )))
                .into_any_element(),
        );
    }
    div()
        .flex()
        .flex_wrap()
        .gap(px(6.0))
        .text_size(px(12.0))
        .text_color(color::muted(cx))
        .children(clauses.into_iter().enumerate().flat_map(|(index, clause)| {
            let separator = (index > 0).then(|| div().child("\u{b7}").into_any_element());
            separator.into_iter().chain(std::iter::once(clause))
        }))
}

/// spent · known costs · elapsed · over, beside the tabs.
pub fn legend(cx: &App) -> impl IntoElement {
    let key = |swatch: AnyElement, label: String| {
        div()
            .flex()
            .items_center()
            .gap(px(5.0))
            .child(swatch)
            .child(label)
    };
    div()
        .flex()
        .items_center()
        .gap(px(14.0))
        .ml(px(20.0))
        .text_size(px(11.0))
        .text_color(color::muted(cx))
        .child(key(
            div()
                .w(px(12.0))
                .h(px(8.0))
                .bg(color::foreground(cx))
                .into_any_element(),
            crate::msg::desktop_budgets_legend_spent(),
        ))
        .child(key(
            div()
                .w(px(12.0))
                .h(px(8.0))
                .border_1()
                .border_color(color::foreground(cx))
                .bg(color::inset_track(cx))
                .into_any_element(),
            crate::msg::desktop_budgets_legend_known(),
        ))
        .child(key(
            div()
                .w(px(2.0))
                .h(px(12.0))
                .bg(color::foreground(cx))
                .into_any_element(),
            crate::msg::desktop_budgets_legend_elapsed(),
        ))
        .child(key(
            div()
                .w(px(12.0))
                .h(px(8.0))
                .bg(color::negative(cx))
                .into_any_element(),
            crate::msg::desktop_budgets_legend_over(),
        ))
}

pub fn render(props: &BudgetsPageProps<'_>, cx: &App) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(20.0))
        .child(stat_strip(props, cx))
        .child(table(props, cx))
        .into_any_element()
}

fn stat_strip(props: &BudgetsPageProps<'_>, cx: &App) -> impl IntoElement {
    let figures = props.figures;
    let budgeted_leaves = figures
        .rows
        .iter()
        .filter(|row| !row.is_parent && row.budget.is_some())
        .count();
    let cell = |first: bool, label: String, figure: AnyElement, details: Vec<AnyElement>| {
        div()
            .flex_1()
            .min_w(px(0.0))
            .px(px(16.0))
            .py(px(12.0))
            .when(!first, |this| {
                this.border_l(px(2.0))
                    .border_color(color::structural_rule(cx))
            })
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(
                div()
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .text_size(px(10.0))
                    .text_color(color::muted(cx))
                    .child(upper(&label)),
            )
            .child(figure)
            .children(details)
    };
    let figure = |value: String, colour: Rgba| {
        div()
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .text_size(px(24.0))
            .text_color(colour)
            .child(value)
            .into_any_element()
    };
    let detail = |value: String| {
        div()
            .text_size(px(11.0))
            .text_color(color::muted(cx))
            .child(value)
            .into_any_element()
    };
    let on_known_click = props.on_known_click.clone();
    let known_hover = color::foreground(cx);
    let left_over = figures.left.0.is_negative();
    let mut budgeted_details = vec![detail(crate::msg::desktop_budgets_stat_budgeted_detail(
        i64::try_from(budgeted_leaves).unwrap_or(i64::MAX),
    ))];
    if figures.carried_in.0.is_positive() {
        budgeted_details.push(detail(crate::msg::desktop_budgets_stat_carried(&text(
            &figures.carried_in,
        ))));
    }
    let left_detail = if left_over {
        detail(crate::msg::desktop_budgets_stat_left_over())
    } else {
        match &figures.per_day {
            Some(per_day) => detail(crate::msg::desktop_budgets_stat_left_per_day(
                &text(per_day),
                i64::from(figures.elapsed.days_left),
            )),
            None => detail(String::new()),
        }
    };
    div()
        .flex()
        .border_2()
        .border_color(color::structural_rule(cx))
        .child(cell(
            true,
            crate::msg::desktop_budgets_stat_budgeted(),
            figure(text(&figures.budgeted), color::foreground(cx)),
            budgeted_details,
        ))
        .child(cell(
            false,
            crate::msg::desktop_budgets_stat_spent(),
            figure(text(&figures.spent), color::foreground(cx)),
            vec![detail(crate::msg::desktop_budgets_stat_spent_detail(
                &percent(&figures.spent, &figures.budgeted).to_string(),
                &i64::from(figures.elapsed.percent).to_string(),
            ))],
        ))
        .child(cell(
            false,
            crate::msg::desktop_budgets_stat_known(),
            figure(text(&figures.known), color::foreground(cx)),
            vec![
                detail(crate::msg::desktop_budgets_stat_known_detail(
                    i64::try_from(figures.known_count).unwrap_or(i64::MAX),
                )),
                div()
                    .id("budgets-known-schedule")
                    .cursor_pointer()
                    .text_size(px(11.0))
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .hover(move |style| style.text_color(known_hover))
                    .on_click(move |_event, window, cx| on_known_click(window, cx))
                    .child(crate::msg::desktop_budgets_stat_known_link())
                    .into_any_element(),
            ],
        ))
        .child(cell(
            false,
            crate::msg::desktop_budgets_stat_left(),
            figure(
                text(&figures.left),
                if left_over {
                    color::accent_text(cx)
                } else {
                    color::foreground(cx)
                },
            ),
            vec![left_detail],
        ))
}

fn table(props: &BudgetsPageProps<'_>, cx: &App) -> AnyElement {
    if props.figures.rows.is_empty() {
        return div()
            .text_color(color::muted(cx))
            .child(crate::msg::desktop_budgets_progress_empty())
            .into_any_element();
    }
    let last = props.figures.rows.len() - 1;
    div()
        .flex()
        .flex_col()
        .border_1()
        .border_color(color::border(cx))
        .child(table_header(cx))
        .children(
            props
                .figures
                .rows
                .iter()
                .enumerate()
                .map(|(index, row)| render_row(index, row, index == last, props, cx)),
        )
        .into_any_element()
}

fn table_header(cx: &App) -> impl IntoElement {
    let right = |width, label: String| {
        div()
            .w(width)
            .flex_none()
            .text_align(gpui::TextAlign::Right)
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
        .child(right(
            BUDGET_WIDTH,
            crate::msg::desktop_budgets_column_budget(),
        ))
        .child(right(
            SPENT_WIDTH,
            crate::msg::desktop_budgets_column_spent(),
        ))
        .child(right(
            KNOWN_WIDTH,
            crate::msg::desktop_budgets_column_known(),
        ))
        .child(right(LEFT_WIDTH, crate::msg::desktop_budgets_column_left()))
        .child(
            div()
                .w(PROGRESS_WIDTH)
                .flex_none()
                .pl(px(20.0))
                .child(upper(&crate::msg::desktop_budgets_column_progress())),
        )
        .child(div().w(ACTIONS_WIDTH).flex_none())
}

/// The row's bar: Spent solid, Known Costs after it, both as shares of the budget (capped at the
/// full width), and the elapsed tick.
pub(super) fn bar(
    row: &CategoryFigures,
    elapsed_percent: u32,
    selected: bool,
    cx: &App,
) -> AnyElement {
    let Some(budget) = row.budget.as_ref().filter(|b| b.0.is_positive()) else {
        return div().into_any_element();
    };
    let share = |part: &Money| {
        (part.0.clone() / budget.0.clone())
            .to_f32()
            .unwrap_or(0.0)
            .clamp(0.0, 1.0)
    };
    let spent = if row.spent.0.is_negative() {
        0.0
    } else {
        share(&row.spent)
    };
    let known = share(&row.known).min(1.0 - spent);
    let ink = if selected {
        color::selection_text(cx)
    } else {
        color::foreground(cx)
    };
    let fill = if row.over { color::negative(cx) } else { ink };
    div()
        .relative()
        .w_full()
        .h(px(12.0))
        .bg(if selected {
            color::selection_muted(cx)
        } else {
            color::inset_track(cx)
        })
        .child(
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left_0()
                .w(relative(spent))
                .bg(fill),
        )
        .when(known > 0.0, |this| {
            this.child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(relative(spent))
                    .w(relative(known))
                    .border_1()
                    .border_color(ink),
            )
        })
        .child(
            div()
                .absolute()
                .top(px(-2.0))
                .bottom(px(-2.0))
                .left(relative(elapsed_percent as f32 / 100.0))
                .w(px(2.0))
                .bg(ink),
        )
        .into_any_element()
}

fn render_row(
    index: usize,
    row: &CategoryFigures,
    last: bool,
    props: &BudgetsPageProps<'_>,
    cx: &App,
) -> AnyElement {
    let selected = props.selected == Some(index);
    let (primary, secondary, accent) = if selected {
        (
            color::selection_text(cx),
            color::selection_muted(cx),
            color::selection_accent_text(cx),
        )
    } else {
        (
            color::foreground(cx),
            color::muted(cx),
            color::accent_text(cx),
        )
    };
    let hover = color::hover(cx);
    let on_row_click = props.on_row_click.clone();
    let on_action_click = props.on_action_click.clone();
    let name = props
        .categories
        .iter()
        .find(|c| c.id == row.category_id)
        .map_or_else(|| EMPTY_CELL.to_string(), |c| c.name.clone());
    let depth = categories::depth(props.categories, row.category_id) as f32;
    let cell = |width, value: String, colour: Rgba| {
        div()
            .w(width)
            .flex_none()
            .text_align(gpui::TextAlign::Right)
            .text_color(colour)
            .child(value)
    };

    let mut name_cell = div()
        .flex_1()
        .min_w(px(140.0))
        .pl(px(depth * INDENT))
        .flex()
        .items_baseline()
        .gap(px(8.0))
        .child(
            div()
                .font_weight(if row.is_parent {
                    gpui::FontWeight::EXTRA_BOLD
                } else {
                    gpui::FontWeight::NORMAL
                })
                .child(if row.is_parent {
                    format!("\u{25be} {name}")
                } else {
                    name
                }),
        );
    if row.is_parent {
        name_cell = name_cell.child(
            div()
                .text_size(px(10.0))
                .text_color(secondary)
                .child(crate::msg::desktop_budgets_row_rollup()),
        );
    } else if row.carried_in.0.is_positive() {
        name_cell = name_cell.child(div().text_size(px(10.0)).text_color(secondary).child(
            crate::msg::desktop_budgets_row_carried(&text(&row.carried_in)),
        ));
    }

    let (budget_cell, left_cell) = match (&row.budget, &row.left) {
        (Some(budget), Some(left)) => {
            let left_cell = if row.over {
                let over = Money(row.spent.0.clone() - budget.0.clone());
                cell(
                    LEFT_WIDTH,
                    crate::msg::desktop_budgets_row_over(&text(&over)),
                    accent,
                )
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
            } else if row.at_risk {
                cell(
                    LEFT_WIDTH,
                    format!(
                        "{} {}",
                        text(left),
                        crate::msg::desktop_budgets_row_at_risk()
                    ),
                    accent,
                )
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
            } else {
                cell(LEFT_WIDTH, text(left), primary)
            };
            (cell(BUDGET_WIDTH, text(budget), primary), left_cell)
        }
        _ => (
            cell(
                BUDGET_WIDTH,
                crate::msg::desktop_budgets_row_unbudgeted(),
                secondary,
            ),
            cell(LEFT_WIDTH, EMPTY_CELL.to_string(), secondary),
        ),
    };

    div()
        .id(SharedString::from(format!(
            "budgets-row-{}",
            row.category_id
        )))
        .debug_selector(move || format!("budgets-progress-row-{index}"))
        .cursor_pointer()
        .flex()
        .items_center()
        .px(px(16.0))
        .py(px(9.0))
        .text_color(primary)
        .when(selected, |this| this.bg(color::selection_background(cx)))
        .when(!selected, |this| this.hover(move |style| style.bg(hover)))
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::hairline(cx))
        })
        .on_click(move |_event, window, cx| on_row_click(index, window, cx))
        .child(name_cell)
        .child(budget_cell)
        .child(cell(SPENT_WIDTH, text(&row.spent), primary))
        .child(cell(
            KNOWN_WIDTH,
            if row.known.0.is_zero() {
                EMPTY_CELL.to_string()
            } else {
                text(&row.known)
            },
            secondary,
        ))
        .child(left_cell)
        .child(div().w(PROGRESS_WIDTH).flex_none().pl(px(20.0)).child(bar(
            row,
            props_elapsed(props),
            selected,
            cx,
        )))
        .child(
            div()
                .id(SharedString::from(format!(
                    "budgets-row-action-{}",
                    row.category_id
                )))
                .debug_selector(move || format!("budgets-progress-action-{index}"))
                .w(ACTIONS_WIDTH)
                .flex_none()
                .text_align(gpui::TextAlign::Right)
                .text_size(px(11.0))
                .text_color(secondary)
                .when(!row.is_parent, |this| {
                    this.on_click(move |_event, window, cx| {
                        // The row's own click would open the detail underneath.
                        cx.stop_propagation();
                        on_action_click(index, window, cx);
                    })
                })
                .child(if row.is_parent {
                    String::new()
                } else if row.budget.is_some() {
                    crate::msg::desktop_budgets_row_edit()
                } else {
                    crate::msg::desktop_budgets_row_set()
                }),
        )
        .into_any_element()
}

fn props_elapsed(props: &BudgetsPageProps<'_>) -> u32 {
    let PeriodFigures { elapsed, .. } = props.figures;
    elapsed.percent
}
