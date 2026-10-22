//! Draws the **8a** Schedule tab (`docs/ux/desktop-mockups/12-bills/README.md`), which absorbed
//! 8f's History tab (#381): the filter row and stat callout (`filters`), then the rows in one
//! bordered table (BILL / ACCOUNT / PLANNED / ACTUAL / DUE / PAID / STATUS / ACTIONS) and a
//! footnote, with the header's meta line. Every figure, label and action comes from the view
//! model; this only lays it out. The selected row inverts, as in the handoff.

use std::rc::Rc;

use gpui::{AnyElement, App, Rgba, SharedString, Window, div, prelude::*, px};
use lib_bills::BillStatus;
use lib_bills::presenter::{LineActions, MetaClause, ScheduleLine, ScheduleViewModel};

use super::{OnPlainClick, OnRowClick, filters};
use crate::form::select::SelectState;
use crate::settings::display::StatusGlyphs;
use crate::theme::color;

pub(super) struct Handlers {
    pub on_row_click: OnRowClick,
    pub on_pay_click: OnRowClick,
    pub on_skip_click: OnRowClick,
    pub on_view_transaction_click: OnRowClick,
    pub filters: filters::Handlers,
}

const BILL_MIN_WIDTH: gpui::Pixels = px(160.0);
const ACCOUNT_WIDTH: gpui::Pixels = px(120.0);
const PLANNED_WIDTH: gpui::Pixels = px(110.0);
const ACTUAL_WIDTH: gpui::Pixels = px(100.0);
const DUE_WIDTH: gpui::Pixels = px(90.0);
const PAID_WIDTH: gpui::Pixels = px(90.0);
const STATUS_WIDTH: gpui::Pixels = px(140.0);
const ACTIONS_WIDTH: gpui::Pixels = px(200.0);

/// A meta line's clauses, `·`-separated; a `strong` segment bold, in the accent on an alert
/// clause.
pub(super) fn meta_line(clauses: &[MetaClause], cx: &App) -> AnyElement {
    let last = clauses.len().saturating_sub(1);
    div()
        .flex()
        .flex_wrap()
        .items_baseline()
        .gap(px(4.0))
        .text_size(px(11.5))
        .text_color(color::faint_text(cx))
        .children(clauses.iter().enumerate().flat_map(|(index, clause)| {
            let strong = if clause.alert {
                color::accent_text(cx)
            } else {
                color::foreground(cx)
            };
            let segments = div().flex().children(clause.segments.iter().map(|segment| {
                match segment.tag.as_deref() {
                    Some("strong") => div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_color(strong)
                        .child(segment.text.clone())
                        .into_any_element(),
                    _ => div().child(segment.text.clone()).into_any_element(),
                }
            }));
            let separator = (index < last).then(|| div().child("\u{b7}").into_any_element());
            std::iter::once(segments.into_any_element()).chain(separator)
        }))
        .into_any_element()
}

pub(super) fn render(
    view: &ScheduleViewModel,
    filter_states: &[SelectState],
    glyphs: StatusGlyphs,
    handlers: Handlers,
    cx: &App,
) -> AnyElement {
    let rows = table(view, glyphs, &handlers, cx);
    div()
        .flex()
        .flex_col()
        .child(filters::render(
            &view.filters,
            view.stats.as_ref(),
            filter_states,
            handlers.filters,
            cx,
        ))
        .child(rows)
        .child(
            div()
                .mt(px(10.0))
                .text_size(px(11.0))
                .text_color(color::faint_text(cx))
                .child(view.footnote.clone()),
        )
        .children(view.history_footnote.as_ref().map(|footnote| {
            div()
                .mt(px(6.0))
                .text_size(px(11.0))
                .text_color(color::faint_text(cx))
                .child(footnote.clone())
        }))
        .into_any_element()
}

fn table(
    view: &ScheduleViewModel,
    glyphs: StatusGlyphs,
    handlers: &Handlers,
    cx: &App,
) -> AnyElement {
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
                .children(view.rows.iter().enumerate().map(|(index, row)| {
                    render_row(index, row, index == last, glyphs, handlers, cx)
                })),
        )
        .into_any_element()
}

fn table_header(view: &ScheduleViewModel, cx: &App) -> impl IntoElement {
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
                .min_w(BILL_MIN_WIDTH)
                .child(columns.bill.clone()),
        )
        .child(cell(ACCOUNT_WIDTH, &columns.account))
        .child(right(PLANNED_WIDTH, &columns.planned))
        .child(right(ACTUAL_WIDTH, &columns.actual))
        .child(
            div()
                .w(DUE_WIDTH)
                .flex_none()
                // The handoff runs DUE flush against PLANNED; a gap keeps them apart.
                .pl(px(16.0))
                .child(columns.due.clone()),
        )
        .child(cell(PAID_WIDTH, &columns.paid))
        .child(cell(STATUS_WIDTH, &columns.status))
        .child(right(ACTIONS_WIDTH, &columns.actions))
}

/// The row's colours: inverted when selected, faint for a row with nothing to do (Skipped,
/// Upcoming), plain otherwise.
struct RowColours {
    text: Rgba,
    secondary: Rgba,
    accent: Rgba,
    button_border: Rgba,
}

impl RowColours {
    fn new(selected: bool, quiet: bool, cx: &App) -> Self {
        if selected {
            Self {
                text: color::selection_text(cx),
                secondary: color::selection_muted(cx),
                accent: color::selection_accent_text(cx),
                button_border: color::selection_muted(cx),
            }
        } else {
            Self {
                text: if quiet {
                    color::faint_text(cx)
                } else {
                    color::foreground(cx)
                },
                secondary: if quiet {
                    color::faint_text(cx)
                } else {
                    color::muted(cx)
                },
                accent: color::accent_text(cx),
                button_border: color::border(cx),
            }
        }
    }
}

fn render_row(
    index: usize,
    row: &ScheduleLine,
    last: bool,
    glyphs: StatusGlyphs,
    handlers: &Handlers,
    cx: &App,
) -> AnyElement {
    let colours = RowColours::new(row.selected, row.quiet, cx);
    let hover = color::hover(cx);
    let on_row_click = handlers.on_row_click.clone();
    let overdue = row.status == BillStatus::Overdue;

    div()
        .id(SharedString::from(format!(
            "bills-schedule-row-{}-{}",
            row.id.plan_id, row.id.due
        )))
        .debug_selector(move || format!("bills-row-{index}"))
        .cursor_pointer()
        .flex()
        .items_center()
        .px(px(16.0))
        .py(px(10.0))
        .text_color(colours.text)
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::hairline(cx))
        })
        .when(row.selected, |this| {
            this.bg(color::selection_background(cx))
        })
        .when(!row.selected, |this| {
            this.hover(move |style| style.bg(hover))
        })
        .on_click(move |_event, window, cx| on_row_click(index, window, cx))
        .child(
            div()
                .flex_1()
                .min_w(BILL_MIN_WIDTH)
                .flex()
                .items_baseline()
                .gap(px(4.0))
                .child(
                    div()
                        .truncate()
                        .when(row.selected, |this| {
                            this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                        })
                        .child(row.name.clone()),
                )
                .children(row.estimated.as_ref().map(|estimated| {
                    div()
                        .flex_none()
                        .text_color(colours.secondary)
                        .child(format!("\u{b7} {estimated}"))
                })),
        )
        .child(
            div()
                .w(ACCOUNT_WIDTH)
                .flex_none()
                .truncate()
                .text_color(colours.secondary)
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
                .w(ACTUAL_WIDTH)
                .flex_none()
                .flex()
                .justify_end()
                .whitespace_nowrap()
                .child(row.actual.clone()),
        )
        .child(
            div()
                .w(DUE_WIDTH)
                .flex_none()
                .pl(px(16.0))
                .whitespace_nowrap()
                .when(overdue, |this| {
                    this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_color(colours.accent)
                })
                .child(row.due.clone()),
        )
        .child(
            div()
                .w(PAID_WIDTH)
                .flex_none()
                .whitespace_nowrap()
                .text_color(colours.secondary)
                .child(row.paid.clone()),
        )
        .child(
            div()
                .w(STATUS_WIDTH)
                .flex_none()
                .flex()
                .child(status_cell(row, &colours, glyphs, cx)),
        )
        .child(
            div()
                .w(ACTIONS_WIDTH)
                .flex_none()
                .flex()
                .justify_end()
                .gap(px(6.0))
                .children(actions_cell(index, row, &colours, handlers, cx)),
        )
        .into_any_element()
}

/// A row's status: a pill for Overdue (accent) and Due (neutral), flagged while in Needs
/// Attention; plain text for the rest.
fn status_cell(
    row: &ScheduleLine,
    colours: &RowColours,
    glyphs: StatusGlyphs,
    cx: &App,
) -> AnyElement {
    let flag = if row.needs_attention {
        format!("{} ", crate::view::format::flag_glyph(glyphs))
    } else {
        String::new()
    };
    let pill = |background: Rgba, text: Rgba| {
        div()
            .px(px(7.0))
            .py(px(2.0))
            .bg(background)
            .text_color(text)
            .text_size(px(10.5))
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .child(format!("{flag}{}", row.status_label))
            .into_any_element()
    };
    match row.status {
        BillStatus::Overdue => {
            let (background, text) = if row.selected {
                (color::accent(cx), color::selection_text(cx))
            } else {
                (color::accent_tint(cx), color::accent_text(cx))
            };
            pill(background, text)
        }
        BillStatus::Due => {
            let (background, text) = if row.selected {
                (color::selection_muted(cx), color::selection_background(cx))
            } else {
                (color::chrome(cx), color::foreground(cx))
            };
            pill(background, text)
        }
        BillStatus::Paid | BillStatus::Skipped | BillStatus::Upcoming => div()
            .text_size(px(11.5))
            .text_color(colours.secondary)
            .child(row.status_label.clone())
            .into_any_element(),
    }
}

fn actions_cell(
    index: usize,
    row: &ScheduleLine,
    colours: &RowColours,
    handlers: &Handlers,
    cx: &App,
) -> Vec<AnyElement> {
    let id = |kind: &str| SharedString::from(format!("bills-{kind}-{index}"));
    let bind = |handler: &OnRowClick| -> OnPlainClick {
        let handler = handler.clone();
        Rc::new(move |window: &mut Window, cx: &mut App| handler(index, window, cx))
    };
    match &row.actions {
        LineActions::PayOrSkip { pay, skip } => {
            // `pay` is the filled one; on the inverted row it flips to the light fill.
            let (pay_background, pay_text) = if row.selected {
                (color::selection_text(cx), color::selection_background(cx))
            } else {
                (color::foreground(cx), color::background(cx))
            };
            vec![
                action_button(
                    id("pay"),
                    pay.clone(),
                    Some((pay_background, pay_text)),
                    colours.button_border,
                    bind(&handlers.on_pay_click),
                    cx,
                ),
                action_button(
                    id("skip"),
                    skip.clone(),
                    None,
                    colours.button_border,
                    bind(&handlers.on_skip_click),
                    cx,
                ),
            ]
        }
        LineActions::ViewTransaction(label) => vec![action_button(
            id("view"),
            label.clone(),
            None,
            colours.button_border,
            bind(&handlers.on_view_transaction_click),
            cx,
        )],
        LineActions::Note(note) => vec![
            div()
                .text_size(px(11.0))
                .text_color(colours.secondary)
                .child(note.clone())
                .into_any_element(),
        ],
    }
}

/// `padding:4px 9px; font-size:11px; border:1px`, filled when `fill` is given. Stops the click
/// reaching the row.
fn action_button(
    id: SharedString,
    label: String,
    fill: Option<(Rgba, Rgba)>,
    border: Rgba,
    on_click: OnPlainClick,
    cx: &App,
) -> AnyElement {
    let hover = color::hover(cx);
    div()
        .debug_selector({
            let id = id.clone();
            move || id.to_string()
        })
        .id(id)
        .cursor_pointer()
        .py(px(4.0))
        .px(px(9.0))
        .border_1()
        .border_color(border)
        .text_size(px(11.0))
        .when_some(fill, |this, (background, text)| {
            this.bg(background)
                .border_color(background)
                .text_color(text)
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
        })
        .when(fill.is_none(), |this| {
            this.hover(move |style| style.bg(hover))
        })
        .on_click(move |_event, window, cx| {
            cx.stop_propagation();
            on_click(window, cx)
        })
        .child(label)
        .into_any_element()
}
