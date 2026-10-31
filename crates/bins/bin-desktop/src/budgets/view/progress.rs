//! The **9a** Progress tab (`docs/ux/desktop-mockups/14-budgets-v2/README.md`): the legend, the
//! four-cell stat strip and the Category table, drawn from the Presenter's
//! [`ProgressViewModel`].
//!
//! A row's bar is solid for Spent, then a bordered muted segment for Known Costs (the handoff
//! hatches it; GPUI has no pattern fill, and the outline keeps it from resting on colour alone),
//! with a 2px tick at the period's elapsed share. An over-budget bar fills in the negative colour.
//! The selected row inverts.

use gpui::{AnyElement, App, Entity, Rgba, SharedString, div, prelude::*, px, relative};
use lib_budgets::presenter::{
    BarView, BudgetsPageEvent, ProgressEvent, ProgressRow, ProgressViewModel, StatView,
};
use lib_locale::format::upper;

use super::on_event;
use super::page::BudgetsPage;
use crate::theme::color;

const BUDGET_WIDTH: gpui::Pixels = px(96.0);
const SPENT_WIDTH: gpui::Pixels = px(96.0);
const KNOWN_WIDTH: gpui::Pixels = px(96.0);
const LEFT_WIDTH: gpui::Pixels = px(130.0);
const PROGRESS_WIDTH: gpui::Pixels = px(190.0);
const ACTIONS_WIDTH: gpui::Pixels = px(60.0);
const INDENT: f32 = 18.0;

/// spent · known costs · elapsed · over, beside the tabs.
pub(super) fn legend(labels: &[String; 4], cx: &App) -> impl IntoElement {
    let key = |swatch: AnyElement, label: String| {
        div()
            .flex()
            .items_center()
            .gap(px(5.0))
            .child(swatch)
            .child(label)
    };
    let [spent, known, elapsed, over] = labels.clone();
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
            spent,
        ))
        .child(key(
            div()
                .w(px(12.0))
                .h(px(8.0))
                .border_1()
                .border_color(color::foreground(cx))
                .bg(color::inset_track(cx))
                .into_any_element(),
            known,
        ))
        .child(key(
            div()
                .w(px(2.0))
                .h(px(12.0))
                .bg(color::foreground(cx))
                .into_any_element(),
            elapsed,
        ))
        .child(key(
            div()
                .w(px(12.0))
                .h(px(8.0))
                .bg(color::negative(cx))
                .into_any_element(),
            over,
        ))
}

pub(super) fn render(
    view: &ProgressViewModel,
    entity: &Entity<BudgetsPage>,
    cx: &App,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(20.0))
        .child(stat_strip(&view.stats, entity, cx))
        .child(table(view, entity, cx))
        .into_any_element()
}

fn stat_strip(stats: &[StatView], entity: &Entity<BudgetsPage>, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .border_2()
        .border_color(color::structural_rule(cx))
        .children(stats.iter().enumerate().map(|(index, stat)| {
            let detail = |value: String| {
                div()
                    .text_size(px(11.0))
                    .text_color(color::muted(cx))
                    .child(value)
                    .into_any_element()
            };
            let link = stat.link.clone().map(|link| {
                let on_click = on_event(
                    entity,
                    BudgetsPageEvent::Progress(ProgressEvent::KnownCosts),
                );
                let hover = color::foreground(cx);
                div()
                    .id("budgets-known-schedule")
                    .debug_selector(|| "budgets-known-schedule".to_string())
                    .cursor_pointer()
                    .text_size(px(11.0))
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .hover(move |style| style.text_color(hover))
                    .on_click(move |_event, window, cx| on_click(window, cx))
                    .child(link)
                    .into_any_element()
            });
            div()
                .flex_1()
                .min_w(px(0.0))
                .px(px(16.0))
                .py(px(12.0))
                .when(index > 0, |this| {
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
                        .child(upper(&stat.label)),
                )
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(24.0))
                        .text_color(if stat.alert {
                            color::accent_text(cx)
                        } else {
                            color::foreground(cx)
                        })
                        .child(stat.figure.clone()),
                )
                .children(stat.details.iter().cloned().map(detail))
                .children(link)
        }))
}

fn table(view: &ProgressViewModel, entity: &Entity<BudgetsPage>, cx: &App) -> AnyElement {
    if let Some(empty) = &view.empty {
        return div()
            .text_color(color::muted(cx))
            .child(empty.clone())
            .into_any_element();
    }
    let last = view.rows.len().saturating_sub(1);
    div()
        .flex()
        .flex_col()
        .border_1()
        .border_color(color::border(cx))
        .child(table_header(view, cx))
        .children(view.rows.iter().enumerate().map(|(index, row)| {
            render_row(
                index,
                row,
                index == last,
                view.selected == Some(index),
                view.elapsed,
                entity,
                cx,
            )
        }))
        .into_any_element()
}

fn table_header(view: &ProgressViewModel, cx: &App) -> impl IntoElement {
    let columns = &view.columns;
    let right = |width, label: &str| {
        div()
            .w(width)
            .flex_none()
            .text_align(gpui::TextAlign::Right)
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
        .child(right(BUDGET_WIDTH, &columns.budget))
        .child(right(SPENT_WIDTH, &columns.spent))
        .child(right(KNOWN_WIDTH, &columns.known))
        .child(right(LEFT_WIDTH, &columns.left))
        .child(
            div()
                .w(PROGRESS_WIDTH)
                .flex_none()
                .pl(px(20.0))
                .child(upper(&columns.progress)),
        )
        .child(div().w(ACTIONS_WIDTH).flex_none())
}

/// The row's bar: Spent solid, Known Costs after it, and the elapsed tick at `elapsed` (0 to 1).
/// Shared with the Category detail.
pub(crate) fn bar(bar: Option<BarView>, elapsed: f32, selected: bool, cx: &App) -> AnyElement {
    let Some(bar) = bar else {
        return div().into_any_element();
    };
    let ink = if selected {
        color::selection_text(cx)
    } else {
        color::foreground(cx)
    };
    let fill = if bar.over { color::negative(cx) } else { ink };
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
                .w(relative(bar.spent))
                .bg(fill),
        )
        .when(bar.known > 0.0, |this| {
            this.child(
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(relative(bar.spent))
                    .w(relative(bar.known))
                    .border_1()
                    .border_color(ink),
            )
        })
        .child(
            div()
                .absolute()
                .top(px(-2.0))
                .bottom(px(-2.0))
                .left(relative(elapsed))
                .w(px(2.0))
                .bg(ink),
        )
        .into_any_element()
}

fn render_row(
    index: usize,
    row: &ProgressRow,
    last: bool,
    selected: bool,
    elapsed: f32,
    entity: &Entity<BudgetsPage>,
    cx: &App,
) -> AnyElement {
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
    let on_row_click = on_event(
        entity,
        BudgetsPageEvent::Progress(ProgressEvent::Select(index)),
    );
    let on_action_click = on_event(
        entity,
        BudgetsPageEvent::Progress(ProgressEvent::Action(index)),
    );
    let cell = |width, value: String, colour: Rgba| {
        div()
            .w(width)
            .flex_none()
            .text_align(gpui::TextAlign::Right)
            .text_color(colour)
            .child(value)
    };
    let name_cell = div()
        .flex_1()
        .min_w(px(140.0))
        .pl(px(row.depth as f32 * INDENT))
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
                .child(row.name.clone()),
        )
        .when_some(row.note.clone(), |this, note| {
            this.child(div().text_size(px(10.0)).text_color(secondary).child(note))
        });
    let left_cell = cell(
        LEFT_WIDTH,
        row.left.clone(),
        if row.left_alert {
            accent
        } else if row.budgeted {
            primary
        } else {
            secondary
        },
    )
    .when(row.left_alert, |this| {
        this.font_weight(gpui::FontWeight::EXTRA_BOLD)
    });
    div()
        .id(SharedString::from(format!("budgets-progress-row-{index}")))
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
        .on_click(move |_event, window, cx| on_row_click(window, cx))
        .child(name_cell)
        .child(cell(
            BUDGET_WIDTH,
            row.budget.clone(),
            if row.budgeted { primary } else { secondary },
        ))
        .child(cell(SPENT_WIDTH, row.spent.clone(), primary))
        .child(cell(KNOWN_WIDTH, row.known.clone(), secondary))
        .child(left_cell)
        .child(
            div()
                .w(PROGRESS_WIDTH)
                .flex_none()
                .pl(px(20.0))
                .child(bar(row.bar, elapsed, selected, cx)),
        )
        .child(
            div()
                .id(SharedString::from(format!(
                    "budgets-progress-action-{index}"
                )))
                .debug_selector(move || format!("budgets-progress-action-{index}"))
                .w(ACTIONS_WIDTH)
                .flex_none()
                .text_align(gpui::TextAlign::Right)
                .text_size(px(11.0))
                .text_color(secondary)
                .when(row.action.is_some(), |this| {
                    this.on_click(move |_event, window, cx| {
                        // The row's own click would open the detail underneath.
                        cx.stop_propagation();
                        on_action_click(window, cx);
                    })
                })
                .child(row.action.clone().unwrap_or_default()),
        )
        .into_any_element()
}
