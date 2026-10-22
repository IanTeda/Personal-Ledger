//! Draws the **8b** Planner tab (`docs/ux/desktop-mockups/12-bills/README.md`): the Bill Plans in
//! one bordered table (NAME / CATEGORY / ACCOUNT / PLANNED / RECURS / LEAD / ACTIVE / ACTIONS) and
//! a footnote, with the header's meta line. A Plan with an Attention Lead is tinted to draw the eye
//! to LEAD; an inactive Plan is muted throughout; the selected row inverts, as on the Schedule.

use gpui::{AnyElement, App, Rgba, SharedString, Window, div, prelude::*, px};
use lib_bills::presenter::{PlanLine, PlannerViewModel};

use super::OnRowClick;
use crate::theme::color;

pub(super) struct Handlers {
    pub on_row_click: OnRowClick,
    pub on_edit_click: OnRowClick,
}

const NAME_MIN_WIDTH: gpui::Pixels = px(160.0);
const CATEGORY_WIDTH: gpui::Pixels = px(130.0);
const ACCOUNT_WIDTH: gpui::Pixels = px(110.0);
const PLANNED_WIDTH: gpui::Pixels = px(110.0);
const RECURS_WIDTH: gpui::Pixels = px(110.0);
const LEAD_WIDTH: gpui::Pixels = px(60.0);
const ACTIVE_WIDTH: gpui::Pixels = px(60.0);
const ACTIONS_WIDTH: gpui::Pixels = px(70.0);

/// `10 bill plans · 1 inactive`, the inactive count bold.
pub(super) fn meta_line(view: &PlannerViewModel, cx: &App) -> AnyElement {
    div()
        .flex()
        .flex_wrap()
        .items_baseline()
        .gap(px(4.0))
        .text_size(px(11.5))
        .text_color(color::faint_text(cx))
        .child(div().child(view.meta_plans.clone()))
        .when_some(view.meta_inactive.as_ref(), |this, inactive| {
            this.child(div().child("\u{b7}"))
                .child(div().flex().children(inactive.iter().map(|segment| {
                    match segment.tag.as_deref() {
                        Some("strong") => div()
                            .font_weight(gpui::FontWeight::EXTRA_BOLD)
                            .text_color(color::foreground(cx))
                            .child(segment.text.clone())
                            .into_any_element(),
                        _ => div().child(segment.text.clone()).into_any_element(),
                    }
                })))
        })
        .into_any_element()
}

pub(super) fn render(view: &PlannerViewModel, handlers: Handlers, cx: &App) -> AnyElement {
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
        .child(
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
                        .map(|(index, row)| render_row(index, row, index == last, &handlers, cx)),
                ),
        )
        .child(
            div()
                .mt(px(10.0))
                .text_size(px(11.0))
                .text_color(color::faint_text(cx))
                .child(view.footnote.clone()),
        )
        .into_any_element()
}

fn table_header(view: &PlannerViewModel, cx: &App) -> impl IntoElement {
    let columns = &view.columns;
    let cell = |width, label: &String| div().w(width).flex_none().child(label.clone());
    let right = |width, label: &String| {
        div()
            .w(width)
            .flex_none()
            .text_align(gpui::TextAlign::Right)
            .child(label.clone())
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
                .min_w(NAME_MIN_WIDTH)
                .child(columns.name.clone()),
        )
        .child(cell(CATEGORY_WIDTH, &columns.category))
        .child(cell(ACCOUNT_WIDTH, &columns.account))
        .child(right(PLANNED_WIDTH, &columns.planned))
        .child(
            div()
                .w(RECURS_WIDTH)
                .flex_none()
                // The handoff runs RECURS flush against PLANNED; a gap keeps them apart.
                .pl(px(16.0))
                .child(columns.recurs.clone()),
        )
        .child(cell(LEAD_WIDTH, &columns.lead))
        .child(cell(ACTIVE_WIDTH, &columns.active))
        .child(right(ACTIONS_WIDTH, &columns.actions))
}

fn render_row(
    index: usize,
    row: &PlanLine,
    last: bool,
    handlers: &Handlers,
    cx: &App,
) -> AnyElement {
    let (text, secondary, accent, button_border): (Rgba, Rgba, Rgba, Rgba) = if row.selected {
        (
            color::selection_text(cx),
            color::selection_muted(cx),
            color::selection_accent_text(cx),
            color::selection_muted(cx),
        )
    } else if row.inactive {
        let faint = color::faint_text(cx);
        (faint, faint, faint, color::hairline(cx))
    } else {
        (
            color::foreground(cx),
            color::muted(cx),
            color::accent_text(cx),
            color::border(cx),
        )
    };
    let hover = color::hover(cx);
    let on_row_click = handlers.on_row_click.clone();
    let on_edit_click = handlers.on_edit_click.clone();
    let edit_hover = color::hover(cx);
    let id = row.id;

    div()
        .id(SharedString::from(format!("bills-plan-row-{id}")))
        .debug_selector(move || format!("bills-plan-row-{id}"))
        .cursor_pointer()
        .flex()
        .items_center()
        .px(px(16.0))
        .py(px(10.0))
        .text_color(text)
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::hairline(cx))
        })
        .when(row.selected, |this| {
            this.bg(color::selection_background(cx))
        })
        .when(!row.selected && row.has_lead, |this| {
            this.bg(color::accent_tint(cx))
        })
        .when(!row.selected, |this| {
            this.hover(move |style| style.bg(hover))
        })
        .on_click(move |_event, window, cx| on_row_click(index, window, cx))
        .child(
            div()
                .flex_1()
                .min_w(NAME_MIN_WIDTH)
                .truncate()
                .when(row.selected, |this| {
                    this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                })
                .child(row.name.clone()),
        )
        .child(
            div()
                .w(CATEGORY_WIDTH)
                .flex_none()
                .truncate()
                .text_color(secondary)
                .child(row.category.clone()),
        )
        .child(
            div()
                .w(ACCOUNT_WIDTH)
                .flex_none()
                .truncate()
                .text_color(secondary)
                .child(row.account.clone()),
        )
        .child(
            div()
                .w(PLANNED_WIDTH)
                .flex_none()
                .flex()
                .justify_end()
                .whitespace_nowrap()
                .child(row.planned.clone()),
        )
        .child(
            div()
                .w(RECURS_WIDTH)
                .flex_none()
                .pl(px(16.0))
                .whitespace_nowrap()
                .text_color(secondary)
                .child(row.recurs.clone()),
        )
        .child(
            div()
                .w(LEAD_WIDTH)
                .flex_none()
                .whitespace_nowrap()
                .when(row.has_lead, |this| {
                    this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_color(accent)
                })
                .when(!row.has_lead, |this| this.text_color(secondary))
                .child(row.lead.clone()),
        )
        .child(
            div()
                .w(ACTIVE_WIDTH)
                .flex_none()
                .text_color(secondary)
                .child(row.active.clone()),
        )
        .child(
            div()
                .w(ACTIONS_WIDTH)
                .flex_none()
                .flex()
                .justify_end()
                .child(
                    div()
                        .id(SharedString::from(format!("bills-plan-edit-{id}")))
                        .debug_selector(move || format!("bills-plan-edit-{id}"))
                        .cursor_pointer()
                        .py(px(4.0))
                        .px(px(9.0))
                        .border_1()
                        .border_color(button_border)
                        .text_size(px(11.0))
                        .hover(move |style| style.bg(edit_hover))
                        .on_click(move |_event, window: &mut Window, cx| {
                            cx.stop_propagation();
                            on_edit_click(index, window, cx)
                        })
                        .child(row.edit.clone()),
                ),
        )
        .into_any_element()
}
