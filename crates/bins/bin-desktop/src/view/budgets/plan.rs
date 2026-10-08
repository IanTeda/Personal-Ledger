//! The **9b** Plan tab (`docs/ux/desktop/14-budgets-v2/README.md`): a six-month grid of Budget
//! Amounts per leaf Category under their parent's label, a ROLLOVER column, and the Total
//! budgeted and Unallocated footers.
//!
//! A cell that starts a record is bold, closed months are muted and read-only, the current month's
//! header is filled, and the cursor cell inverts. The cell being typed into shows its text with a
//! caret inside an accent border. Every input is a callback into `Shell`.

use std::rc::Rc;

use bigdecimal::Signed;
use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};
use lib_core::Money;
use lib_locale::format::{format_month, upper};

use crate::{
    budgets::{PLAN_MONTHS, PLAN_ROLLOVER_COLUMN, Plan, PlanCell, PlanEdit, PlanRow, Rollover},
    period::Period,
    theme::color,
    transactions::rows::EMPTY_CELL,
    view::format::amount,
};

const MONTH_WIDTH: gpui::Pixels = px(92.0);
const ROLLOVER_WIDTH: gpui::Pixels = px(150.0);

/// Called with a cell's row and column (`PLAN_ROLLOVER_COLUMN` for the ROLLOVER cell).
pub type OnCellClick = Rc<dyn Fn(usize, usize, &mut Window, &mut App)>;

pub struct PlanProps<'a> {
    pub plan: &'a Plan,
    pub current: Period,
    /// The cursor's row and column.
    pub cursor: (usize, usize),
    pub edit: Option<&'a PlanEdit>,
    pub on_cell_click: OnCellClick,
}

/// `Jul – Dec 2026`, or with both years when the range crosses one.
pub fn range_label(plan: &Plan) -> String {
    let (Some(first), Some(last)) = (plan.months.first(), plan.months.last()) else {
        return String::new();
    };
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

fn rollover_label(rollover: Rollover) -> String {
    match rollover {
        Rollover::None => EMPTY_CELL.to_string(),
        Rollover::CarryUnspent => crate::msg::desktop_budgets_plan_rollover_unspent(),
        Rollover::CarryBoth => crate::msg::desktop_budgets_plan_rollover_both(),
    }
}

pub fn render(props: &PlanProps<'_>, cx: &App) -> AnyElement {
    if props.plan.row_count() == 0 {
        return div()
            .text_color(color::muted(cx))
            .child(crate::msg::desktop_budgets_plan_empty())
            .into_any_element();
    }
    let mut next_row = 0;
    let mut body: Vec<AnyElement> = Vec::new();
    for section in &props.plan.sections {
        let label = section
            .parent
            .clone()
            .unwrap_or_else(crate::msg::desktop_budgets_plan_other);
        body.push(section_label(&label, cx));
        for row in &section.rows {
            body.push(render_row(next_row, row, props, cx));
            next_row += 1;
        }
    }
    div()
        .flex()
        .flex_col()
        .border_1()
        .border_color(color::border(cx))
        .child(header(props, cx))
        .children(body)
        .child(footer(props, cx))
        .into_any_element()
}

fn header(props: &PlanProps<'_>, cx: &App) -> impl IntoElement {
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
        .children(props.plan.months.iter().map(|month| {
            let now = *month == props.current;
            let mut label = upper(&format_month(month.month));
            if now {
                label = format!(
                    "{label} \u{b7} {}",
                    upper(&crate::msg::desktop_budgets_plan_now())
                );
            }
            div()
                .w(MONTH_WIDTH)
                .flex_none()
                .py(px(2.0))
                .px(px(6.0))
                .text_align(gpui::TextAlign::Right)
                .whitespace_nowrap()
                .when(now, |this| {
                    this.bg(color::selection_background(cx))
                        .text_color(color::selection_text(cx))
                })
                .child(label)
        }))
        .child(
            div()
                .w(ROLLOVER_WIDTH)
                .flex_none()
                .pl(px(20.0))
                .child(upper(&crate::msg::desktop_budgets_plan_column_rollover())),
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

fn render_row(index: usize, row: &PlanRow, props: &PlanProps<'_>, cx: &App) -> AnyElement {
    let cursor_here = props.cursor.0 == index;
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
                .when(cursor_here, |this| {
                    this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                })
                .child(row.name.clone()),
        )
        .children(
            row.cells
                .iter()
                .enumerate()
                .map(|(column, cell)| render_cell(index, column, row, cell, props, cx)),
        )
        .child(rollover_cell(index, row, props, cx))
        .into_any_element()
}

fn render_cell(
    index: usize,
    column: usize,
    row: &PlanRow,
    cell: &PlanCell,
    props: &PlanProps<'_>,
    cx: &App,
) -> AnyElement {
    let on_click = props.on_cell_click.clone();
    let cursor_here = props.cursor == (index, column);
    let editing = props
        .edit
        .filter(|edit| edit.category_id == row.category_id && edit.month == cell.month);
    let text = cell
        .amount
        .as_ref()
        .map_or_else(|| EMPTY_CELL.to_string(), |money| amount(money).1);
    let base = div()
        .id(SharedString::from(format!("budgets-plan-{index}-{column}")))
        .debug_selector(move || format!("budgets-plan-{index}-{column}"))
        .w(MONTH_WIDTH)
        .flex_none()
        .py(px(4.0))
        .px(px(6.0))
        .text_align(gpui::TextAlign::Right)
        .cursor_pointer()
        .on_click(move |_event, window, cx| on_click(index, column, window, cx));
    if let Some(edit) = editing {
        return base
            .border_2()
            .border_color(color::accent(cx))
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .child(format!("{}\u{258f}", edit.text))
            .into_any_element();
    }
    base.border_2()
        .border_color(gpui::transparent_black())
        .when(cell.own_record, |this| {
            this.font_weight(gpui::FontWeight::EXTRA_BOLD)
        })
        .when(cell.closed, |this| this.text_color(color::muted(cx)))
        .when(cursor_here, |this| {
            this.bg(color::selection_background(cx))
                .text_color(color::selection_text(cx))
        })
        .child(text)
        .into_any_element()
}

fn rollover_cell(index: usize, row: &PlanRow, props: &PlanProps<'_>, cx: &App) -> AnyElement {
    let on_click = props.on_cell_click.clone();
    let cursor_here = props.cursor == (index, PLAN_ROLLOVER_COLUMN);
    div()
        .id(SharedString::from(format!("budgets-plan-{index}-rollover")))
        .w(ROLLOVER_WIDTH)
        .flex_none()
        .py(px(4.0))
        .px(px(6.0))
        .ml(px(14.0))
        .cursor_pointer()
        .text_color(color::muted(cx))
        .when(cursor_here, |this| {
            this.bg(color::selection_background(cx))
                .text_color(color::selection_text(cx))
        })
        .on_click(move |_event, window, cx| on_click(index, PLAN_ROLLOVER_COLUMN, window, cx))
        .child(row.rollover.map_or_else(String::new, rollover_label))
        .into_any_element()
}

/// Total budgeted, then Unallocated of the average income, one figure per month.
fn footer(props: &PlanProps<'_>, cx: &App) -> AnyElement {
    let line = |label: String, figures: Vec<(String, bool)>| {
        div()
            .flex()
            .items_center()
            .px(px(16.0))
            .py(px(6.0))
            .border_t(px(1.0))
            .border_color(color::border(cx))
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .child(div().flex_1().min_w(px(140.0)).child(label))
            .children(figures.into_iter().map(|(text, over)| {
                div()
                    .w(MONTH_WIDTH)
                    .flex_none()
                    .px(px(6.0))
                    .text_align(gpui::TextAlign::Right)
                    .when(over, |this| this.text_color(color::accent_text(cx)))
                    .child(text)
            }))
            .child(div().w(ROLLOVER_WIDTH).flex_none().ml(px(14.0)))
    };
    debug_assert_eq!(props.plan.totals.len(), PLAN_MONTHS);
    div()
        .flex()
        .flex_col()
        .child(line(
            crate::msg::desktop_budgets_plan_total(),
            props
                .plan
                .totals
                .iter()
                .map(|total| (amount(total).1, false))
                .collect(),
        ))
        .child(line(
            crate::msg::desktop_budgets_plan_unallocated(&amount(&props.plan.average_income).1),
            props
                .plan
                .unallocated
                .iter()
                .map(|left: &Money| (amount(left).1, left.0.is_negative()))
                .collect(),
        ))
        .into_any_element()
}
