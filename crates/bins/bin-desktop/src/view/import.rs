//! The stubbed **6e** Import "match payees" step (`docs/ux/desktop/20-payees/README.md`): the
//! stepper, heading and subline, a scrolling table (RAW DESCRIPTION / PAYEE / CATEGORY / STATUS /
//! AMOUNT), and a footer bar with the summary, the "remember new payees' rules" checkbox, **back**
//! and **continue**. Shown in place of the Transactions page while `Shell` holds an
//! [`ImportState`]; the pure rules live in `crate::import`.
//!
//! A row resolved by a rule shows its Payee and Category as text, as in the handoff; any other
//! row, and any row whose select is open, shows the two selects. Their lists open inline beneath
//! the cell, the row growing to hold them, like the dialogs' shared select.

use std::rc::Rc;

use gpui::{AnyElement, App, Rgba, ScrollHandle, SharedString, Window, div, prelude::*, px};

use lib_locale::format::upper;

use crate::{
    form::select::SelectState,
    import::{self, ImportRow, ImportState, PayeeChoices, RowPayee, RowSelect, RowStatus},
    payees::{self, Payee, form::PayeeOptions},
    theme::color,
};

const MONOSPACE: &str = "monospace";
const PAYEE_WIDTH: gpui::Pixels = px(150.0);
const CATEGORY_WIDTH: gpui::Pixels = px(130.0);
const STATUS_WIDTH: gpui::Pixels = px(110.0);
const AMOUNT_WIDTH: gpui::Pixels = px(90.0);
/// The gap between the fixed columns, so a select never touches its neighbour.
const COLUMN_GAP: gpui::Pixels = px(8.0);

pub type OnRowClick = Rc<dyn Fn(usize, &mut Window, &mut App)>;
pub type OnSelectClick = Rc<dyn Fn(usize, RowSelect, &mut Window, &mut App)>;
pub type OnOptionClick = Rc<dyn Fn(usize, &mut Window, &mut App)>;
pub type OnClick = Rc<dyn Fn(&mut Window, &mut App)>;

pub struct ImportPageProps<'a> {
    pub state: &'a ImportState,
    pub payees: &'a [Payee],
    /// The Category select's options, "choose category…" first.
    pub categories: &'a PayeeOptions,
    pub on_row_click: OnRowClick,
    pub on_select_click: OnSelectClick,
    pub on_option_click: OnOptionClick,
    pub on_remember_click: OnClick,
    pub on_back_click: OnClick,
    pub on_continue_click: OnClick,
}

/// The create option's label, shared by the select and the closed cell.
pub fn create_label(name: &str) -> String {
    crate::msg::desktop_import_create_payee("+", name)
}

/// Row `raw`'s Payee choices, as the select offers them.
pub fn payee_choices(payees: &[Payee], raw: &str) -> PayeeChoices {
    PayeeChoices::new(payees, raw, create_label)
}

pub fn render(
    focused: bool,
    scroll_handle: &ScrollHandle,
    props: ImportPageProps<'_>,
    cx: &App,
) -> AnyElement {
    div()
        .id("import")
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .flex()
        .flex_col()
        .when(focused, |this| {
            this.border_l(px(2.0)).border_color(color::foreground(cx))
        })
        .child(heading(props.state.rows.len(), cx))
        .child(table_header(cx))
        .child(
            div()
                .id("import-rows")
                .flex_1()
                .min_h(px(0.0))
                .overflow_y_scroll()
                .track_scroll(scroll_handle)
                .children(
                    props
                        .state
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
fn heading(count: usize, cx: &App) -> impl IntoElement {
    let dash = || div().child("\u{2014}");
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
                .child(
                    div()
                        .text_color(color::foreground(cx))
                        .child(crate::msg::desktop_import_step_upload()),
                )
                .child(dash())
                .child(
                    div()
                        .text_color(color::accent_text(cx))
                        .child(crate::msg::desktop_import_step_match()),
                )
                .child(dash())
                .child(crate::msg::desktop_import_step_confirm()),
        )
        .child(
            div()
                .mb(px(4.0))
                .text_size(px(26.0))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .child(crate::msg::desktop_import_title()),
        )
        .child(
            div()
                .text_size(px(11.5))
                .text_color(color::faint_text(cx))
                .child(crate::msg::desktop_import_subline(
                    i64::try_from(count).unwrap_or(i64::MAX),
                    import::STATEMENT_FILE,
                )),
        )
}

/// `padding:8px 28px; font:800 10px; color:#9b9797`.
fn table_header(cx: &App) -> impl IntoElement {
    let fixed = |width, label: String| div().w(width).flex_none().child(upper(&label));
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
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .child(upper(&crate::msg::desktop_import_column_raw())),
        )
        .child(fixed(
            PAYEE_WIDTH,
            crate::msg::desktop_import_column_payee(),
        ))
        .child(fixed(
            CATEGORY_WIDTH,
            crate::msg::desktop_import_column_category(),
        ))
        .child(fixed(
            STATUS_WIDTH,
            crate::msg::desktop_import_column_status(),
        ))
        .child(
            div()
                .w(AMOUNT_WIDTH)
                .flex_none()
                .flex()
                .justify_end()
                .child(upper(&crate::msg::desktop_import_column_amount())),
        )
}

/// What a cell shows when its select is closed.
fn payee_text(row: &ImportRow, payees: &[Payee]) -> Option<String> {
    match row.payee.as_ref()? {
        RowPayee::Rule(id) | RowPayee::Existing(id) => {
            payees::get(payees, *id).map(|p| p.name.clone())
        }
        RowPayee::Create(name) => Some(create_label(name)),
    }
}

fn category_text(row: &ImportRow, categories: &PayeeOptions) -> Option<String> {
    let id = row.category?;
    categories
        .ids
        .iter()
        .position(|candidate| *candidate == Some(id))
        .map(|index| categories.labels[index].clone())
}

fn render_row(index: usize, row: &ImportRow, props: &ImportPageProps<'_>, cx: &App) -> AnyElement {
    let state = props.state;
    let selected = state.selected == index;
    let unresolved = !matches!(row.payee, Some(RowPayee::Rule(_))) || row.category.is_none();
    let open = state
        .open_select
        .as_ref()
        .filter(|(open_row, _, _)| *open_row == index);
    let show_selects = unresolved || open.is_some();
    let on_row_click = props.on_row_click.clone();
    let hover = color::hover(cx);

    let payee_cell = if show_selects {
        let choices = payee_choices(props.payees, &row.raw);
        cell_select(
            SelectCell {
                id: format!("import-payee-{index}"),
                text: payee_text(row, props.payees)
                    .unwrap_or_else(crate::msg::desktop_import_choose_payee),
                missing: row.payee.is_none(),
                open: open
                    .filter(|(_, select, _)| *select == RowSelect::Payee)
                    .map(|(_, _, state)| (state, choices.labels.as_slice())),
                on_click: select_click(props, index, RowSelect::Payee),
                on_option_click: props.on_option_click.clone(),
            },
            cx,
        )
    } else {
        div()
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .truncate()
            .child(payee_text(row, props.payees).unwrap_or_default())
            .into_any_element()
    };
    let category_cell = if show_selects {
        cell_select(
            SelectCell {
                id: format!("import-category-{index}"),
                text: category_text(row, props.categories)
                    .unwrap_or_else(crate::msg::desktop_import_choose_category),
                missing: row.category.is_none(),
                open: open
                    .filter(|(_, select, _)| *select == RowSelect::Category)
                    .map(|(_, _, state)| (state, props.categories.labels.as_slice())),
                on_click: select_click(props, index, RowSelect::Category),
                on_option_click: props.on_option_click.clone(),
            },
            cx,
        )
    } else {
        div()
            .text_color(color::muted(cx))
            .truncate()
            .child(category_text(row, props.categories).unwrap_or_default())
            .into_any_element()
    };
    let (negative, amount) = crate::view::format::amount(&row.amount);

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
        .child(
            div()
                .w(STATUS_WIDTH)
                .flex_none()
                .flex()
                .child(status_tag(row.status(), cx)),
        )
        .child(
            div()
                .w(AMOUNT_WIDTH)
                .flex_none()
                .flex()
                .justify_end()
                .whitespace_nowrap()
                .when(negative, |this| this.text_color(color::negative_text(cx)))
                .child(amount),
        )
        .into_any_element()
}

fn select_click(props: &ImportPageProps<'_>, index: usize, select: RowSelect) -> OnClick {
    let on_select_click = props.on_select_click.clone();
    Rc::new(move |window: &mut Window, cx: &mut App| on_select_click(index, select, window, cx))
}

struct SelectCell<'a> {
    id: String,
    text: String,
    /// Draws the red border: nothing chosen yet.
    missing: bool,
    /// The open list's state and options, when this select is the open one.
    open: Option<(&'a SelectState, &'a [String])>,
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
        .children(open.map(|(state, options)| {
            div()
                .flex()
                .flex_col()
                .bg(color::background(cx))
                .border_1()
                .border_t_0()
                .border_color(color::border(cx))
                .children(state.visible_range(options.len()).map(|option| {
                    let highlighted = option == state.highlight();
                    let on_option_click = on_option_click.clone();
                    div()
                        .id(SharedString::from(format!("{id}-option-{option}")))
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
                        .child(options[option].clone())
                }))
        }))
        .into_any_element()
}

/// `.tag` 10px, `padding:2px 7px`: neutral for a rule match or a hand-picked Payee, accent for a
/// new Payee, a red outline for a row that needs review.
fn status_tag(status: RowStatus, cx: &App) -> impl IntoElement {
    let (label, background, text, border): (String, Option<Rgba>, Rgba, Rgba) = match status {
        RowStatus::RuleMatch => (
            crate::msg::desktop_import_status_rule_match(),
            Some(color::chrome(cx)),
            color::muted(cx),
            color::border(cx),
        ),
        RowStatus::Matched => (
            crate::msg::desktop_import_status_matched(),
            Some(color::chrome(cx)),
            color::muted(cx),
            color::border(cx),
        ),
        RowStatus::NewPayee => (
            crate::msg::desktop_import_status_new_payee(),
            Some(color::accent(cx)),
            color::selection_text(cx),
            color::accent(cx),
        ),
        RowStatus::NeedsReview => (
            crate::msg::desktop_import_status_needs_review(),
            None,
            color::accent_text(cx),
            color::accent(cx),
        ),
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
fn footer(props: &ImportPageProps<'_>, cx: &App) -> impl IntoElement {
    let summary = import::summary(&props.state.rows);
    let count = |n: usize| i64::try_from(n).unwrap_or(i64::MAX);
    let can_continue = props.state.can_continue();
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
                .child(crate::msg::desktop_import_summary_rows(count(summary.rows)))
                .child(dot())
                .child(crate::msg::desktop_import_summary_matched(
                    &summary.rule_matched.to_string(),
                ))
                .child(dot())
                .child(crate::msg::desktop_import_summary_new(count(
                    summary.new_payees,
                )))
                .child(dot())
                .child(
                    div()
                        .when(summary.needs_review > 0, |this| {
                            this.text_color(color::accent_text(cx))
                                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        })
                        .child(crate::msg::desktop_import_summary_review(count(
                            summary.needs_review,
                        ))),
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
                        .when(props.state.remember, |this| {
                            this.bg(color::foreground(cx))
                                .text_color(color::background(cx))
                                .text_size(px(9.0))
                                .child("\u{2713}")
                        }),
                )
                .child(crate::msg::desktop_import_remember()),
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
                .child(crate::msg::desktop_import_back()),
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
                .child(crate::msg::desktop_import_continue())
                .child(div().opacity(0.75).child("enter")),
        )
}
