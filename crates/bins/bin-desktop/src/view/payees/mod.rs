//! The **6a** Payees page (`docs/ux/desktop/Payees/README.md`): a full-width management page -- a
//! header row (title, `N payees · K without a default category`, **+ Add payee**), a 2px rule,
//! then one bordered table (NAME / DEFAULT CATEGORY / MATCH RULES / TRANSACTIONS / TOTAL /
//! ACTIONS) and a footnote. The selected row inverts, as in the handoff.
//!
//! TRANSACTIONS and TOTAL are computed live by `payees::usage` (all time, base Unit only), and the
//! header counts leave inactive Payees out (#283); an inactive row stays listed, dimmed and tagged.
//! Every action is a callback into `Shell`, so the keyboard and the mouse reach the same handlers.

pub mod add_dialog;
pub mod delete_dialog;
pub mod rules_field;

use std::rc::Rc;

use gpui::{AnyElement, App, Rgba, ScrollHandle, SharedString, Window, div, prelude::*, px};

use lib_locale::format::upper;

use crate::{
    accounts::Account,
    categories::Category,
    nav::Noun,
    payees::{self, Payee},
    theme::color,
    transactions::Transaction,
};

/// Called with a Payee's [`Payee::id`].
pub type OnPayeeClick = Rc<dyn Fn(u32, &mut Window, &mut App)>;
pub type OnAddClick = Rc<dyn Fn(&mut Window, &mut App)>;
type OnPlainClick = Rc<dyn Fn(&mut Window, &mut App)>;

pub struct PayeesPageProps<'a> {
    pub payees: &'a [Payee],
    pub categories: &'a [Category],
    pub transactions: &'a [Transaction],
    pub accounts: &'a [Account],
    /// The base Unit's code: TOTAL sums only Splits in it.
    pub base_unit: Option<&'a str>,
    /// Index into `payees` of the selected row.
    pub selected: Option<usize>,
    pub on_add_click: OnAddClick,
    pub on_row_click: OnPayeeClick,
    pub on_edit_click: OnPayeeClick,
    pub on_delete_click: OnPayeeClick,
}

/// The NAME column's floor, so the fixed columns never squeeze it to nothing.
const NAME_MIN_WIDTH: gpui::Pixels = px(140.0);
const CATEGORY_WIDTH: gpui::Pixels = px(130.0);
const RULES_WIDTH: gpui::Pixels = px(100.0);
const TRANSACTIONS_WIDTH: gpui::Pixels = px(100.0);
const TOTAL_WIDTH: gpui::Pixels = px(110.0);
const ACTIONS_WIDTH: gpui::Pixels = px(150.0);

pub fn render(
    focused: bool,
    scroll_handle: &ScrollHandle,
    props: PayeesPageProps<'_>,
    cx: &App,
) -> AnyElement {
    let last = props.payees.len().saturating_sub(1);
    div()
        .id("payees")
        .flex_1()
        .min_w(px(0.0))
        .h_full()
        .overflow_y_scroll()
        .track_scroll(scroll_handle)
        .when(focused, |this| {
            this.border_l(px(2.0)).border_color(color::foreground(cx))
        })
        .px(px(28.0))
        .py(px(22.0))
        .child(page_header(&props, cx))
        .child(
            div()
                .h(px(2.0))
                .flex_none()
                .bg(color::structural_rule(cx))
                .mt(px(24.0))
                .mb(px(24.0)),
        )
        .when(props.payees.is_empty(), |this| {
            this.child(
                div()
                    .text_color(color::muted(cx))
                    .child(crate::msg::desktop_payees_empty("n")),
            )
        })
        .when(!props.payees.is_empty(), |this| {
            this.child(
                div()
                    .flex()
                    .flex_col()
                    .border_1()
                    .border_color(color::border(cx))
                    .child(table_header(cx))
                    .children(props.payees.iter().enumerate().map(|(index, payee)| {
                        row(
                            payee,
                            index == last,
                            props.selected == Some(index),
                            &props,
                            cx,
                        )
                    })),
            )
            .child(
                div()
                    .mt(px(10.0))
                    .text_size(px(11.0))
                    .text_color(color::faint_text(cx))
                    .child(crate::msg::desktop_payees_footnote()),
            )
        })
        .into_any_element()
}

fn page_header(props: &PayeesPageProps<'_>, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .items_end()
        .justify_between()
        .gap(px(16.0))
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(6.0))
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(28.0))
                        .text_color(color::foreground(cx))
                        .child(Noun::Payees.label()),
                )
                .child(summary_line(props.payees, cx)),
        )
        .child(add_button(props.on_add_click.clone(), cx))
}

/// `24 payees · 3 without a default category`, the second count bold in the accent text colour.
fn summary_line(payees: &[Payee], cx: &App) -> impl IntoElement {
    let count = i64::try_from(payees::active_count(payees)).unwrap_or(i64::MAX);
    let without = payees::without_default_category_count(payees).to_string();
    div()
        .flex()
        .flex_wrap()
        .items_baseline()
        .gap(px(4.0))
        .text_size(px(11.5))
        .text_color(color::faint_text(cx))
        .child(crate::msg::desktop_payees_count(count))
        .child("\u{b7}")
        .children(
            crate::msg::desktop_payees_without_default(&without)
                .into_iter()
                .map(|segment| match segment.tag.as_deref() {
                    Some("strong") => div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_color(color::accent_text(cx))
                        .child(segment.text)
                        .into_any_element(),
                    _ => div().child(segment.text).into_any_element(),
                }),
        )
}

fn add_button(on_click: OnAddClick, cx: &App) -> impl IntoElement {
    let hover = color::muted(cx);
    div()
        .id("payees-add")
        .cursor_pointer()
        .flex_none()
        .py(px(10.0))
        .px(px(16.0))
        .bg(color::foreground(cx))
        .text_color(color::selection_text(cx))
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .whitespace_nowrap()
        .hover(move |style| style.bg(hover))
        .on_click(move |_event, window, cx| on_click(window, cx))
        .child(crate::msg::desktop_payees_add_button("+"))
}

/// `padding:10px 16px; background:#eae9e9; font:800 10px; color:#605d5d`.
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
                .min_w(NAME_MIN_WIDTH)
                .child(upper(&lib_locale::msg::column_name())),
        )
        .child(
            div()
                .w(CATEGORY_WIDTH)
                .flex_none()
                .child(upper(&crate::msg::desktop_payees_column_default_category())),
        )
        .child(
            div()
                .w(RULES_WIDTH)
                .flex_none()
                .child(upper(&crate::msg::desktop_payees_column_match_rules())),
        )
        .child(right(
            TRANSACTIONS_WIDTH,
            crate::msg::desktop_payees_column_transactions(),
        ))
        .child(right(
            TOTAL_WIDTH,
            crate::msg::desktop_payees_column_total(),
        ))
        .child(right(ACTIONS_WIDTH, lib_locale::msg::column_actions()))
}

/// The row's colours: inverted when selected (the handoff's `#201e1d` fill), plain otherwise.
struct RowColours {
    text: Rgba,
    secondary: Rgba,
    faint: Rgba,
    missing: Rgba,
    negative: Rgba,
    button_border: Rgba,
}

impl RowColours {
    fn new(selected: bool, cx: &App) -> Self {
        if selected {
            Self {
                text: color::selection_text(cx),
                secondary: color::selection_muted(cx),
                faint: color::selection_muted(cx),
                missing: color::selection_accent_text(cx),
                negative: color::selection_negative_text(cx),
                button_border: color::selection_muted(cx),
            }
        } else {
            Self {
                text: color::foreground(cx),
                secondary: color::muted(cx),
                faint: color::faint_text(cx),
                missing: color::accent_text(cx),
                negative: color::negative_text(cx),
                button_border: color::border(cx),
            }
        }
    }
}

fn row(
    payee: &Payee,
    last: bool,
    selected: bool,
    props: &PayeesPageProps<'_>,
    cx: &App,
) -> impl IntoElement {
    let id = payee.id;
    let colours = RowColours::new(selected, cx);
    let hover = color::hover(cx);
    let usage = payees::usage(props.transactions, props.accounts, props.base_unit, id);
    let (negative, total) = crate::format::amount(&usage.total);
    let category = payee
        .default_category
        .and_then(|category_id| props.categories.iter().find(|c| c.id == category_id));
    let rules = i64::try_from(payee.aliases.len()).unwrap_or(i64::MAX);

    let on_row_click = props.on_row_click.clone();
    let on_edit_click = props.on_edit_click.clone();
    let on_delete_click = props.on_delete_click.clone();

    div()
        .id(SharedString::from(format!("payees-row-{id}")))
        .cursor_pointer()
        .flex()
        .items_center()
        .px(px(16.0))
        .py(px(10.0))
        .text_color(colours.text)
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::hairline(cx))
        })
        .when(selected, |this| this.bg(color::selection_background(cx)))
        .when(!selected, |this| this.hover(move |style| style.bg(hover)))
        // #283: an inactive Payee stays listed, dimmed and tagged, rather than hidden.
        .when(!payee.is_active, |this| this.opacity(0.55))
        .on_click(move |_event, window, cx| on_row_click(id, window, cx))
        .child(
            div()
                .flex_1()
                .min_w(NAME_MIN_WIDTH)
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(
                    div()
                        .truncate()
                        .when(selected, |this| {
                            this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                        })
                        .child(payee.name.clone()),
                )
                .when(!payee.is_active, |this| {
                    this.child(
                        div()
                            .flex_none()
                            .px(px(6.0))
                            .py(px(1.0))
                            .border_1()
                            .border_color(colours.button_border)
                            .text_size(px(10.0))
                            .text_color(colours.secondary)
                            .child(crate::msg::desktop_payees_inactive()),
                    )
                }),
        )
        .child(div().w(CATEGORY_WIDTH).flex_none().truncate().map(|this| {
            match category {
                Some(category) => this
                    .text_color(colours.secondary)
                    .child(category.name.clone()),
                None => this
                    .text_color(colours.missing)
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .child(crate::msg::desktop_payees_no_default_category()),
            }
        }))
        .child(
            div()
                .w(RULES_WIDTH)
                .flex_none()
                .text_color(colours.faint)
                .child(crate::msg::desktop_payees_rule_count(rules)),
        )
        .child(
            div()
                .w(TRANSACTIONS_WIDTH)
                .flex_none()
                .flex()
                .justify_end()
                .child(usage.splits.to_string()),
        )
        .child(
            div()
                .w(TOTAL_WIDTH)
                .flex_none()
                .flex()
                .justify_end()
                .whitespace_nowrap()
                .when(selected, |this| {
                    this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                })
                .when(negative, |this| this.text_color(colours.negative))
                .child(total),
        )
        .child(
            div()
                .w(ACTIONS_WIDTH)
                .flex_none()
                .flex()
                .justify_end()
                .gap(px(6.0))
                .child(row_action_button(
                    SharedString::from(format!("payees-edit-{id}")),
                    crate::msg::desktop_payees_row_edit(),
                    colours.button_border,
                    Rc::new(move |window: &mut Window, cx: &mut App| on_edit_click(id, window, cx)),
                    cx,
                ))
                .child(row_action_button(
                    SharedString::from(format!("payees-delete-{id}")),
                    crate::msg::desktop_payees_row_delete(),
                    colours.button_border,
                    Rc::new(move |window: &mut Window, cx: &mut App| {
                        on_delete_click(id, window, cx)
                    }),
                    cx,
                )),
        )
}

/// `padding:4px 8px; font-size:11px; border:1px`. Stops the click reaching the row, which would
/// otherwise also open Transactions.
fn row_action_button(
    id: SharedString,
    label: String,
    border: Rgba,
    on_click: OnPlainClick,
    cx: &App,
) -> impl IntoElement {
    let hover = color::hover(cx);
    div()
        .id(id)
        .cursor_pointer()
        .py(px(4.0))
        .px(px(8.0))
        .border_1()
        .border_color(border)
        .text_size(px(11.0))
        .hover(move |style| style.bg(hover))
        .on_click(move |_event, window, cx| {
            cx.stop_propagation();
            on_click(window, cx)
        })
        .child(label)
}
