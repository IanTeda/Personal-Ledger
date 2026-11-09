//! The Import "match payees" step's drawing (`docs/ux/desktop-mockups/20-payees/README.md`, 6e):
//! the stepper, heading and subline, a scrolling table (RAW DESCRIPTION / PAYEE / CATEGORY / STATUS
//! / AMOUNT), and a footer bar with the summary, the "remember new payees' rules" checkbox, **back**
//! and **continue**. It draws a borrow of `lib-transaction-import`'s view model and nothing else,
//! each click the Presenter's event sent through [`PageClicks`]; `page.rs` is the step Presenter's
//! `PageView`.
//!
//! A row resolved by a rule shows its Payee and Category as text, as in the handoff; any other
//! row, and any row whose select is open, shows the two selects. Their lists open inline beneath
//! the cell, the row growing to hold them, like the dialogs' shared select.

pub mod page;
pub mod snapshot;

use std::ops::Range;

use gpui::{AnyElement, App, ClickEvent, Rgba, SharedString, div, prelude::*, px};
use lib_locale::format::upper;
use lib_transaction_import::RowStatus;
use lib_transaction_import::presenter::{
    ImportEvent, ImportViewModel, OpenSelectView, RowLine, RowSelect,
};

use crate::{shell::view::page_clicks::PageClicks, theme::color};
use page::ImportPage;

const MONOSPACE: &str = "monospace";
const PAYEE_WIDTH: gpui::Pixels = px(150.0);
const CATEGORY_WIDTH: gpui::Pixels = px(130.0);
const STATUS_WIDTH: gpui::Pixels = px(110.0);
const AMOUNT_WIDTH: gpui::Pixels = px(90.0);
/// The gap between the fixed columns, so a select never touches its neighbour.
const COLUMN_GAP: gpui::Pixels = px(8.0);

/// Draws the open step from `view`, its open select's options from `select_range`.
pub(super) fn render(
    view: &ImportViewModel,
    continue_chord: String,
    select_range: Option<Range<usize>>,
    scroll: &gpui::ScrollHandle,
    clicks: &PageClicks<ImportPage>,
    cx: &App,
) -> AnyElement {
    let rows = div()
        .id("import-rows")
        .flex_1()
        .min_h(px(0.0))
        .overflow_y_scroll()
        .track_scroll(scroll)
        .children(
            view.rows
                .iter()
                .enumerate()
                .map(|(index, row)| render_row(index, row, view, &select_range, clicks, cx)),
        );
    div()
        .id("import")
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .flex()
        .flex_col()
        .child(heading(view, cx))
        .child(table_header(view, cx))
        .child(rows)
        .child(footer(view, continue_chord, clicks, cx))
        .into_any_element()
}

/// The stepper (step 1 done, 2 current, 3 upcoming), the heading and the subline.
fn heading(view: &ImportViewModel, cx: &App) -> impl IntoElement {
    let dash = || div().child("\u{2014}");
    let step = |index: usize| view.steps.get(index).cloned().unwrap_or_default();
    div()
        .flex_none()
        .px(px(28.0))
        .pt(px(16.0))
        .pb(px(14.0))
        .border_b(px(2.0))
        .border_color(color::structural_rule(cx))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(8.0))
                .mb(px(12.0))
                .text_size(px(11.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(color::faint_text(cx))
                .child(div().text_color(color::foreground(cx)).child(step(0)))
                .child(dash())
                .child(div().text_color(color::accent_text(cx)).child(step(1)))
                .child(dash())
                .child(step(2)),
        )
        .child(
            div()
                .mb(px(4.0))
                .text_size(px(26.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .child(view.title.clone()),
        )
        .child(
            div()
                .text_size(px(11.5))
                .text_color(color::faint_text(cx))
                .child(view.subline.clone()),
        )
}

/// `padding:8px 28px; font:800 10px; color:#9b9797`.
fn table_header(view: &ImportViewModel, cx: &App) -> impl IntoElement {
    let columns = &view.columns;
    let fixed = |width, label: &str| div().w(width).flex_none().child(upper(label));
    div()
        .flex_none()
        .flex()
        .gap(COLUMN_GAP)
        .px(px(28.0))
        .py(px(8.0))
        .border_b(px(1.0))
        .border_color(color::hairline(cx))
        .text_size(px(10.0))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_color(color::faint_text(cx))
        .child(div().flex_1().min_w(px(0.0)).child(upper(&columns.raw)))
        .child(fixed(PAYEE_WIDTH, &columns.payee))
        .child(fixed(CATEGORY_WIDTH, &columns.category))
        .child(fixed(STATUS_WIDTH, &columns.status))
        .child(
            div()
                .w(AMOUNT_WIDTH)
                .flex_none()
                .flex()
                .justify_end()
                .child(upper(&columns.amount)),
        )
}

/// Row `index`: a click selects it, and a click on one of its selects opens that select, or closes
/// it when it is the open one.
fn render_row(
    index: usize,
    row: &RowLine,
    view: &ImportViewModel,
    select_range: &Option<Range<usize>>,
    clicks: &PageClicks<ImportPage>,
    cx: &App,
) -> AnyElement {
    let selected = view.selected == index;
    let unresolved = row.status != RowStatus::RuleMatch;
    let open = view.open_select.as_ref().filter(|open| open.row == index);
    let show_selects = unresolved || open.is_some();
    let hover = color::hover(cx);
    let open_list = |which: RowSelect| {
        open.filter(|open| open.which == which)
            .zip(select_range.clone())
    };
    let select_click = |which: RowSelect| {
        if open_list(which).is_some() {
            ImportEvent::Cancel
        } else {
            ImportEvent::OpenRow(index, which)
        }
    };

    let payee_cell = if show_selects {
        cell_select(
            SelectCell {
                id: format!("import-payee-{index}"),
                text: row.payee.clone(),
                missing: !row.payee_chosen,
                open: open_list(RowSelect::Payee),
                event: select_click(RowSelect::Payee),
            },
            clicks,
            cx,
        )
    } else {
        div()
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .truncate()
            .child(row.payee.clone())
            .into_any_element()
    };
    let category_cell = if show_selects {
        cell_select(
            SelectCell {
                id: format!("import-category-{index}"),
                text: row.category.clone(),
                missing: !row.category_chosen,
                open: open_list(RowSelect::Category),
                event: select_click(RowSelect::Category),
            },
            clicks,
            cx,
        )
    } else {
        div()
            .text_color(color::muted(cx))
            .truncate()
            .child(row.category.clone())
            .into_any_element()
    };

    div()
        .debug_selector(move || format!("import-row-{index}"))
        .id(SharedString::from(format!("import-row-{index}")))
        .flex()
        .items_start()
        .gap(COLUMN_GAP)
        .pl(px(26.0))
        .pr(px(28.0))
        .py(px(9.0))
        .border_b(px(1.0))
        .border_color(color::chrome(cx))
        .when(unresolved, |this| this.bg(color::chrome(cx)))
        .when(!unresolved, |this| this.hover(move |style| style.bg(hover)))
        .on_click(clicks.row(ImportEvent::SelectRow(index)))
        // The keyboard's row: a 2px accent bar in the left padding, so nothing shifts.
        .child(
            div()
                .w(px(2.0))
                .mr(-COLUMN_GAP)
                .h(px(18.0))
                .flex_none()
                .when(selected, |this| this.bg(color::accent(cx))),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .pt(px(2.0))
                .truncate()
                .font_family(MONOSPACE)
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .child(row.raw.clone()),
        )
        .child(div().w(PAYEE_WIDTH).flex_none().child(payee_cell))
        .child(div().w(CATEGORY_WIDTH).flex_none().child(category_cell))
        .child(div().w(STATUS_WIDTH).flex_none().flex().child(status_tag(
            row.status,
            row.status_label.clone(),
            cx,
        )))
        .child(
            div()
                .w(AMOUNT_WIDTH)
                .flex_none()
                .flex()
                .justify_end()
                .whitespace_nowrap()
                .when(row.negative, |this| {
                    this.text_color(color::negative_text(cx))
                })
                .child(row.amount.clone()),
        )
        .into_any_element()
}

struct SelectCell<'a> {
    id: String,
    text: String,
    /// Draws the red border: nothing chosen yet.
    missing: bool,
    /// The open select and the options of it to draw, when this select is the open one.
    open: Option<(&'a OpenSelectView, Range<usize>)>,
    /// What a click on the select sends.
    event: ImportEvent,
}

/// `padding:5px 6px; border:1px; font-size:12px`, red-bordered while `missing`.
fn cell_select(cell: SelectCell<'_>, clicks: &PageClicks<ImportPage>, cx: &App) -> AnyElement {
    let SelectCell {
        id,
        text,
        missing,
        open,
        event,
    } = cell;
    let on_click = clicks.send(event);
    let border = if missing || open.is_some() {
        color::accent(cx)
    } else {
        color::border(cx)
    };
    div()
        .w_full()
        .child(
            div()
                .id(SharedString::from(id.clone()))
                .debug_selector({
                    let id = id.clone();
                    move || id
                })
                .cursor_pointer()
                .flex()
                .items_center()
                .justify_between()
                .gap(px(4.0))
                .py(px(5.0))
                .px(px(6.0))
                .border_1()
                .border_color(border)
                .bg(color::background(cx))
                .text_size(px(12.0))
                .on_click(move |event: &ClickEvent, window, cx| {
                    cx.stop_propagation();
                    on_click(event, window, cx);
                })
                .child(div().min_w(px(0.0)).truncate().child(text))
                .child(
                    div()
                        .flex_none()
                        .text_size(px(10.0))
                        .text_color(color::faint_text(cx))
                        .child(if open.is_some() {
                            "\u{25b4}"
                        } else {
                            "\u{25be}"
                        }),
                ),
        )
        .children(open.map(|(open, range)| {
            let options = &open.select.options;
            div()
                .flex()
                .flex_col()
                .bg(color::background(cx))
                .border_1()
                .border_t_0()
                .border_color(color::border(cx))
                .children(range.map(|option| {
                    let highlighted = option == open.highlight;
                    let on_option_click = clicks.send(ImportEvent::Choose(option));
                    div()
                        .id(SharedString::from(format!("{id}-option-{option}")))
                        .debug_selector({
                            let id = id.clone();
                            move || format!("{id}-option-{option}")
                        })
                        .cursor_pointer()
                        .py(px(4.0))
                        .px(px(6.0))
                        .text_size(px(12.0))
                        .truncate()
                        .when(highlighted, |this| {
                            this.bg(color::selection_background(cx))
                                .text_color(color::selection_text(cx))
                        })
                        .on_click(move |event: &ClickEvent, window, cx| {
                            cx.stop_propagation();
                            on_option_click(event, window, cx);
                        })
                        .child(options[option].label.clone())
                }))
        }))
        .into_any_element()
}

/// `.tag` 10px, `padding:2px 7px`: neutral for a rule match or a hand-picked Payee, accent for a
/// new Payee, a red outline for a row that needs review.
fn status_tag(status: RowStatus, label: String, cx: &App) -> impl IntoElement {
    let (background, text, border): (Option<Rgba>, Rgba, Rgba) = match status {
        RowStatus::RuleMatch | RowStatus::Matched => {
            (Some(color::chrome(cx)), color::muted(cx), color::border(cx))
        }
        RowStatus::NewPayee => (
            Some(color::accent(cx)),
            color::selection_text(cx),
            color::accent(cx),
        ),
        RowStatus::NeedsReview => (None, color::accent_text(cx), color::accent(cx)),
    };
    div()
        .mt(px(2.0))
        .px(px(7.0))
        .py(px(2.0))
        .border_1()
        .border_color(border)
        .when_some(background, |this, background| this.bg(background))
        .text_size(px(10.0))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_color(text)
        .child(label)
}

/// The footer bar: the summary, the remember checkbox, **back** and **continue** (disabled while
/// any row needs review), with the chord **continue** is on beside it.
fn footer(
    view: &ImportViewModel,
    continue_chord: String,
    clicks: &PageClicks<ImportPage>,
    cx: &App,
) -> impl IntoElement {
    let summary = &view.summary;
    let can_continue = view.can_continue;
    let needs_review = view
        .rows
        .iter()
        .any(|row| row.status == RowStatus::NeedsReview);
    let on_continue_click = clicks.send(ImportEvent::Continue);
    let hover = color::hover(cx);
    let dot = || div().child("\u{b7}");

    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(14.0))
        .px(px(28.0))
        .py(px(11.0))
        .border_t(px(2.0))
        .border_color(color::structural_rule(cx))
        .bg(color::chrome(cx))
        .text_size(px(11.5))
        .text_color(color::muted(cx))
        .child(
            div()
                .flex()
                .flex_wrap()
                .gap(px(4.0))
                .child(summary.rows.clone())
                .child(dot())
                .child(summary.matched.clone())
                .child(dot())
                .child(summary.new_payees.clone())
                .child(dot())
                .child(
                    div()
                        .when(needs_review, |this| {
                            this.text_color(color::accent_text(cx))
                                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        })
                        .child(summary.needs_review.clone()),
                ),
        )
        .child(
            div()
                .debug_selector(|| "import-remember".to_string())
                .id("import-remember")
                .cursor_pointer()
                .flex()
                .items_center()
                .gap(px(6.0))
                .on_click(clicks.send(ImportEvent::ToggleRemember))
                .child(
                    div()
                        .size(px(12.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .border_1()
                        .border_color(color::foreground(cx))
                        .when(view.remember, |this| {
                            this.bg(color::foreground(cx))
                                .text_color(color::background(cx))
                                .text_size(px(9.0))
                                .child("\u{2713}")
                        }),
                )
                .child(view.remember_label.clone()),
        )
        .child(div().flex_1())
        .child(
            div()
                .debug_selector(|| "import-back".to_string())
                .id("import-back")
                .cursor_pointer()
                .py(px(8.0))
                .px(px(16.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(color::foreground(cx))
                .hover(move |style| style.bg(hover))
                .on_click(clicks.send(ImportEvent::Back))
                .child(view.back.clone()),
        )
        .child(
            div()
                .debug_selector(|| "import-continue".to_string())
                .id("import-continue")
                .flex()
                .gap(px(4.0))
                .py(px(8.0))
                .px(px(16.0))
                .bg(color::foreground(cx))
                .text_color(color::selection_text(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .map(|this| {
                    if can_continue {
                        this.cursor_pointer().on_click(on_continue_click)
                    } else {
                        this.opacity(0.38)
                    }
                })
                .child(view.continue_label.clone())
                .child(div().opacity(0.75).child(continue_chord)),
        )
}
