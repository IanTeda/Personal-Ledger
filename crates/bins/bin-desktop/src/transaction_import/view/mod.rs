//! The Import "match payees" step's drawing (`docs/ux/desktop-mockups/20-payees/README.md`, 6e):
//! the stepper, heading and subline, a scrolling table (RAW DESCRIPTION / PAYEE / CATEGORY / STATUS
//! / AMOUNT), and a footer bar with the summary, the "remember new payees' rules" checkbox, **back**
//! and **continue**. It draws `lib-transaction-import`'s view model and nothing else; `page.rs`
//! holds the Presenter.
//!
//! A row resolved by a rule shows its Payee and Category as text, as in the handoff; any other
//! row, and any row whose select is open, shows the two selects. Their lists open inline beneath
//! the cell, the row growing to hold them, like the dialogs' shared select.

pub mod page;
pub mod snapshot;

use std::ops::Range;
use std::rc::Rc;

use gpui::{
    AnyElement, App, Context, Rgba, ScrollHandle, SharedString, Window, div, prelude::*, px,
};
use lib_locale::format::upper;
use lib_transaction_import::RowStatus;
use lib_transaction_import::presenter::{ImportViewModel, OpenSelectView, RowLine, RowSelect};

use crate::theme::color;
use page::ImportPage;

const MONOSPACE: &str = "monospace";
const PAYEE_WIDTH: gpui::Pixels = px(150.0);
const CATEGORY_WIDTH: gpui::Pixels = px(130.0);
const STATUS_WIDTH: gpui::Pixels = px(110.0);
const AMOUNT_WIDTH: gpui::Pixels = px(90.0);
/// The gap between the fixed columns, so a select never touches its neighbour.
const COLUMN_GAP: gpui::Pixels = px(8.0);

type OnRowClick = Rc<dyn Fn(usize, &mut Window, &mut App)>;
type OnSelectClick = Rc<dyn Fn(usize, RowSelect, &mut Window, &mut App)>;
type OnOptionClick = Rc<dyn Fn(usize, &mut Window, &mut App)>;
type OnClick = Rc<dyn Fn(&mut Window, &mut App)>;

struct ImportPageProps {
    view: ImportViewModel,
    /// The open select's options to draw, its window onto the highlight.
    select_range: Option<Range<usize>>,
    /// The chord **continue** is on, from the keymap.
    continue_chord: String,
    on_row_click: OnRowClick,
    on_select_click: OnSelectClick,
    on_option_click: OnOptionClick,
    on_remember_click: OnClick,
    on_back_click: OnClick,
    on_continue_click: OnClick,
}

impl Render for ImportPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<'_, Self>) -> impl IntoElement {
        let Some(view) = self.view_model() else {
            return gpui::Empty.into_any_element();
        };
        let entity = cx.entity();
        let plain = |handler: fn(&mut ImportPage, &mut Context<'_, ImportPage>)| {
            let entity = entity.clone();
            let on_click: OnClick = Rc::new(move |_window, cx| entity.update(cx, handler));
            on_click
        };
        let indexed = |handler: fn(&mut ImportPage, usize, &mut Context<'_, ImportPage>)| {
            let entity = entity.clone();
            let on_click: OnRowClick = Rc::new(move |index, _window, cx| {
                entity.update(cx, |page, cx| handler(page, index, cx));
            });
            on_click
        };
        let on_select_click: OnSelectClick = {
            let entity = entity.clone();
            Rc::new(move |index, which: RowSelect, _window, cx| {
                entity.update(cx, |page, cx| page.click_select(index, which, cx));
            })
        };
        let select_range = view
            .open_select
            .as_ref()
            .map(|open| self.select_range(open.select.options.len()));
        render(
            &self.scroll,
            ImportPageProps {
                view,
                select_range,
                continue_chord: self.continue_chord(),
                on_row_click: indexed(ImportPage::click_row),
                on_select_click,
                on_option_click: indexed(ImportPage::click_option),
                on_remember_click: plain(ImportPage::click_remember),
                on_back_click: plain(ImportPage::click_back),
                on_continue_click: plain(ImportPage::click_continue),
            },
            cx,
        )
    }
}

fn render(scroll_handle: &ScrollHandle, props: ImportPageProps, cx: &App) -> AnyElement {
    div()
        .id("import")
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .flex()
        .flex_col()
        .child(heading(&props.view, cx))
        .child(table_header(&props.view, cx))
        .child(
            div()
                .id("import-rows")
                .flex_1()
                .min_h(px(0.0))
                .overflow_y_scroll()
                .track_scroll(scroll_handle)
                .children(
                    props
                        .view
                        .rows
                        .iter()
                        .enumerate()
                        .map(|(index, row)| render_row(index, row, &props, cx)),
                ),
        )
        .child(footer(&props, cx))
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

fn render_row(index: usize, row: &RowLine, props: &ImportPageProps, cx: &App) -> AnyElement {
    let view = &props.view;
    let selected = view.selected == index;
    let unresolved = row.status != RowStatus::RuleMatch;
    let open = view.open_select.as_ref().filter(|open| open.row == index);
    let show_selects = unresolved || open.is_some();
    let on_row_click = props.on_row_click.clone();
    let hover = color::hover(cx);
    let open_list = |which: RowSelect| {
        open.filter(|open| open.which == which)
            .zip(props.select_range.clone())
    };

    let payee_cell = if show_selects {
        cell_select(
            SelectCell {
                id: format!("import-payee-{index}"),
                text: row.payee.clone(),
                missing: !row.payee_chosen,
                open: open_list(RowSelect::Payee),
                on_click: select_click(props, index, RowSelect::Payee),
                on_option_click: props.on_option_click.clone(),
            },
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
                on_click: select_click(props, index, RowSelect::Category),
                on_option_click: props.on_option_click.clone(),
            },
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
        .on_click(move |_event, window, cx| on_row_click(index, window, cx))
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

fn select_click(props: &ImportPageProps, index: usize, select: RowSelect) -> OnClick {
    let on_select_click = props.on_select_click.clone();
    Rc::new(move |window: &mut Window, cx: &mut App| on_select_click(index, select, window, cx))
}

struct SelectCell<'a> {
    id: String,
    text: String,
    /// Draws the red border: nothing chosen yet.
    missing: bool,
    /// The open select and the options of it to draw, when this select is the open one.
    open: Option<(&'a OpenSelectView, Range<usize>)>,
    on_click: OnClick,
    on_option_click: OnOptionClick,
}

/// `padding:5px 6px; border:1px; font-size:12px`, red-bordered while `missing`.
fn cell_select(cell: SelectCell<'_>, cx: &App) -> AnyElement {
    let SelectCell {
        id,
        text,
        missing,
        open,
        on_click,
        on_option_click,
    } = cell;
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
                .on_click(move |_event, window, cx| {
                    cx.stop_propagation();
                    on_click(window, cx)
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
                    let on_option_click = on_option_click.clone();
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
                        .on_click(move |_event, window, cx| {
                            cx.stop_propagation();
                            on_option_click(option, window, cx)
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
/// any row needs review).
fn footer(props: &ImportPageProps, cx: &App) -> impl IntoElement {
    let view = &props.view;
    let summary = &view.summary;
    let can_continue = view.can_continue;
    let needs_review = view
        .rows
        .iter()
        .any(|row| row.status == RowStatus::NeedsReview);
    let on_remember_click = props.on_remember_click.clone();
    let on_back_click = props.on_back_click.clone();
    let on_continue_click = props.on_continue_click.clone();
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
                .on_click(move |_event, window, cx| on_remember_click(window, cx))
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
                .on_click(move |_event, window, cx| on_back_click(window, cx))
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
                        this.cursor_pointer()
                            .on_click(move |_event, window, cx| on_continue_click(window, cx))
                    } else {
                        this.opacity(0.38)
                    }
                })
                .child(view.continue_label.clone())
                .child(div().opacity(0.75).child(props.continue_chord.clone())),
        )
}
