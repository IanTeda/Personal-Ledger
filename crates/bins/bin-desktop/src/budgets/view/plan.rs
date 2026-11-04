//! The **9b** Plan tab (`docs/ux/desktop-mockups/14-budgets-v2/README.md`): a six-month grid of
//! Budget Amounts per leaf Category under their parent's label, a ROLLOVER column, and the Total
//! budgeted and Unallocated footers, drawn from the Presenter's [`PlanViewModel`].
//!
//! A cell that starts a record is bold, closed months are muted and read-only, the current month's
//! header is filled, and the cursor cell inverts. The cell being typed into shows its text with a
//! caret inside an accent border.

use gpui::{AnyElement, App, SharedString, div, prelude::*, px};
use lib_budgets::PLAN_ROLLOVER_COLUMN;
use lib_budgets::presenter::{
    BudgetsPageEvent, PlanCellView, PlanEvent, PlanFooterView, PlanRowView, PlanViewModel,
};
use lib_locale::format::upper;

use super::page::BudgetsPage;
use crate::shell::view::page_clicks::PageClicks;
use crate::theme::color;

const MONTH_WIDTH: gpui::Pixels = px(92.0);
const ROLLOVER_WIDTH: gpui::Pixels = px(150.0);

pub(super) fn render(
    view: &PlanViewModel,
    clicks: &PageClicks<BudgetsPage>,
    cx: &App,
) -> AnyElement {
    if let Some(empty) = &view.empty {
        return div()
            .text_color(color::muted(cx))
            .child(empty.clone())
            .into_any_element();
    }
    let mut body: Vec<AnyElement> = Vec::new();
    for section in &view.sections {
        body.push(section_label(&section.label, cx));
        body.extend(section.rows.iter().map(|row| render_row(row, clicks, cx)));
    }
    div()
        .flex()
        .flex_col()
        .border_1()
        .border_color(color::border(cx))
        .child(header(view, cx))
        .children(body)
        .child(footer_line(&view.total, cx))
        .child(footer_line(&view.unallocated, cx))
        .into_any_element()
}

fn header(view: &PlanViewModel, cx: &App) -> impl IntoElement {
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
                .child(upper(&view.category_column)),
        )
        .children(view.months.iter().map(|month| {
            div()
                .w(MONTH_WIDTH)
                .flex_none()
                .py(px(2.0))
                .px(px(6.0))
                .text_align(gpui::TextAlign::Right)
                .whitespace_nowrap()
                .when(month.now, |this| {
                    this.bg(color::selection_background(cx))
                        .text_color(color::selection_text(cx))
                })
                .child(upper(&month.label))
        }))
        .child(
            div()
                .w(ROLLOVER_WIDTH)
                .flex_none()
                .pl(px(20.0))
                .child(upper(&view.rollover_column)),
        )
}

fn section_label(label: &str, cx: &App) -> AnyElement {
    div()
        .px(px(16.0))
        .pt(px(12.0))
        .pb(px(4.0))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::muted(cx))
        .child(upper(label))
        .into_any_element()
}

fn render_row(row: &PlanRowView, clicks: &PageClicks<BudgetsPage>, cx: &App) -> AnyElement {
    let index = row.index;
    div()
        .flex()
        .items_center()
        .px(px(16.0))
        .py(px(2.0))
        .border_t(px(1.0))
        .border_color(color::hairline(cx))
        .child(
            div()
                .flex_1()
                .min_w(px(140.0))
                .pl(px(10.0))
                .when(row.cursor, |this| {
                    this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                })
                .child(row.name.clone()),
        )
        .children(
            row.cells
                .iter()
                .enumerate()
                .map(|(column, cell)| render_cell(index, column, cell, clicks, cx)),
        )
        .child(rollover_cell(index, &row.rollover, clicks, cx))
        .into_any_element()
}

fn render_cell(
    index: usize,
    column: usize,
    cell: &PlanCellView,
    clicks: &PageClicks<BudgetsPage>,
    cx: &App,
) -> AnyElement {
    let on_click = clicks.send(BudgetsPageEvent::Plan(PlanEvent::ClickCell(index, column)));
    let base = div()
        .id(SharedString::from(format!("budgets-plan-{index}-{column}")))
        .debug_selector(move || format!("budgets-plan-{index}-{column}"))
        .w(MONTH_WIDTH)
        .flex_none()
        .py(px(4.0))
        .px(px(6.0))
        .text_align(gpui::TextAlign::Right)
        .cursor_pointer()
        .on_click(on_click);
    if cell.editing {
        return base
            .border_2()
            .border_color(color::accent(cx))
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .child(format!("{}\u{258f}", cell.text))
            .into_any_element();
    }
    base.border_2()
        .border_color(gpui::transparent_black())
        .when(cell.own_record, |this| {
            this.font_weight(gpui::FontWeight::EXTRA_BOLD)
        })
        .when(cell.closed, |this| this.text_color(color::muted(cx)))
        .when(cell.cursor, |this| {
            this.bg(color::selection_background(cx))
                .text_color(color::selection_text(cx))
        })
        .child(cell.text.clone())
        .into_any_element()
}

fn rollover_cell(
    index: usize,
    cell: &PlanCellView,
    clicks: &PageClicks<BudgetsPage>,
    cx: &App,
) -> AnyElement {
    let on_click = clicks.send(BudgetsPageEvent::Plan(PlanEvent::ClickCell(
        index,
        PLAN_ROLLOVER_COLUMN,
    )));
    div()
        .id(SharedString::from(format!("budgets-plan-{index}-rollover")))
        .debug_selector(move || format!("budgets-plan-{index}-rollover"))
        .w(ROLLOVER_WIDTH)
        .flex_none()
        .py(px(4.0))
        .px(px(6.0))
        .ml(px(14.0))
        .cursor_pointer()
        .text_color(color::muted(cx))
        .when(cell.cursor, |this| {
            this.bg(color::selection_background(cx))
                .text_color(color::selection_text(cx))
        })
        .on_click(on_click)
        .child(cell.text.clone())
        .into_any_element()
}

/// Total budgeted, or Unallocated of the average income: one figure per month.
fn footer_line(line: &PlanFooterView, cx: &App) -> AnyElement {
    div()
        .flex()
        .items_center()
        .px(px(16.0))
        .py(px(6.0))
        .border_t(px(1.0))
        .border_color(color::border(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .child(div().flex_1().min_w(px(140.0)).child(line.label.clone()))
        .children(line.figures.iter().map(|(text, over)| {
            div()
                .w(MONTH_WIDTH)
                .flex_none()
                .px(px(6.0))
                .text_align(gpui::TextAlign::Right)
                .when(*over, |this| this.text_color(color::accent_text(cx)))
                .child(text.clone())
        }))
        .child(div().w(ROLLOVER_WIDTH).flex_none().ml(px(14.0)))
        .into_any_element()
}
