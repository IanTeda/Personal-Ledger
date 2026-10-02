//! The **Payees** page (`docs/ux/desktop/Settings/README.md`, to the 2j pattern): a kicker (A–Z)
//! with **+ Add payee** over one bordered list of name, default category and match rules, with
//! `edit · delete` per row. Usage lives in Transactions and Reports, so there are no transaction
//! or total columns. It reuses the Payees model and dialogs.
//!
//! Every action is a callback into `Shell`, so the keyboard (`n`/`e`/`d`) and the mouse reach the
//! same handlers. A row click only selects: the Transactions hand-off stays on the old page.

use std::rc::Rc;

use gpui::{AnyElement, App, Rgba, SharedString, Window, div, prelude::*, px};
use lib_locale::format::upper;

use crate::{
    categories::Category,
    payees::{self, Payee},
    theme::color,
};

type OnPlainClick = Rc<dyn Fn(&mut Window, &mut App)>;
pub type OnPayeeClick = Rc<dyn Fn(u32, &mut Window, &mut App)>;

pub struct PayeesPageProps<'a> {
    pub payees: &'a [Payee],
    pub categories: &'a [Category],
    /// The selected row's Payee id.
    pub selected: Option<u32>,
    pub on_add_click: OnPlainClick,
    pub on_row_click: OnPayeeClick,
    pub on_edit_click: OnPayeeClick,
    pub on_delete_click: OnPayeeClick,
}

/// The NAME column's floor, so the fixed columns never squeeze it to nothing.
const NAME_MIN_WIDTH: gpui::Pixels = px(140.0);
const CATEGORY_WIDTH: gpui::Pixels = px(150.0);
const RULES_WIDTH: gpui::Pixels = px(90.0);

/// The heading's meta as plain text, for the status line: `24 payees · 3 without default category`.
pub fn scope_text(payees: &[Payee]) -> String {
    format!(
        "{} \u{b7} {}",
        crate::msg::desktop_payees_count(count(payees::active_count(payees))),
        crate::msg::desktop_payees_status_without_default(
            &payees::without_default_category_count(payees).to_string()
        ),
    )
}

/// A row count as the number a plural selector takes.
fn count(len: usize) -> i64 {
    i64::try_from(len).unwrap_or(i64::MAX)
}

/// The heading's meta: the count, then the without-default clause with its number bold in the
/// accent text colour.
pub fn scope_note(payees: &[Payee], cx: &App) -> AnyElement {
    div()
        .flex()
        .items_baseline()
        .gap(px(4.0))
        .child(crate::msg::desktop_payees_count(count(
            payees::active_count(payees),
        )))
        .child("\u{b7}")
        .children(
            crate::msg::desktop_payees_without_default(
                &payees::without_default_category_count(payees).to_string(),
            )
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
        .into_any_element()
}

pub fn render(props: &PayeesPageProps<'_>, focused: bool, cx: &App) -> AnyElement {
    let sorted = payees::sorted_by_name(props.payees);
    let last = sorted.len().saturating_sub(1);
    div()
        .flex()
        .flex_col()
        .child(kicker_row(props.on_add_click.clone(), cx))
        .when(sorted.is_empty(), |this| {
            this.child(
                div()
                    .text_color(color::muted(cx))
                    .child(crate::msg::desktop_payees_empty("n")),
            )
        })
        .when(!sorted.is_empty(), |this| {
            this.child(
                div()
                    .flex()
                    .flex_col()
                    .border_1()
                    .border_color(color::border(cx))
                    .mb(px(18.0))
                    .child(table_header(cx))
                    .children(sorted.iter().enumerate().map(|(position, payee)| {
                        row(
                            payee,
                            position == last,
                            props.selected == Some(payee.id),
                            focused,
                            props,
                            cx,
                        )
                    })),
            )
        })
        .child(
            div()
                .max_w(px(620.0))
                .text_size(px(11.5))
                .text_color(color::muted(cx))
                .child(crate::msg::desktop_payees_footnote()),
        )
        .into_any_element()
}

fn kicker_row(on_add_click: OnPlainClick, cx: &App) -> impl IntoElement {
    let hover = color::muted(cx);
    div()
        .flex()
        .items_center()
        .justify_between()
        .mb(px(10.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .text_color(color::faint_text(cx))
                .child(upper(&crate::msg::desktop_settings_payees_kicker())),
        )
        // `btn btn-primary` at `height:32px`.
        .child(
            div()
                .debug_selector(|| "settings-payees-add".to_string())
                .id("settings-payees-add")
                .cursor_pointer()
                .flex_none()
                .py(px(8.0))
                .px(px(14.0))
                .bg(color::foreground(cx))
                .text_color(color::selection_text(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .whitespace_nowrap()
                .hover(move |style| style.bg(hover))
                .on_click(move |_event, window, cx| on_add_click(window, cx))
                .child(crate::msg::desktop_payees_add_button("+")),
        )
}

/// `padding:6px 14px; background:#eae9e9`: NAME, DEFAULT CATEGORY, MATCH RULES and ACTIONS.
fn table_header(cx: &App) -> impl IntoElement {
    div()
        .flex()
        .px(px(14.0))
        .py(px(6.0))
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
        .child(div().child(upper(&lib_locale::msg::column_actions())))
}

/// The selected row is inverted while the page has focus (`background:#201e1d; color:#f3f2f2`)
/// and only tinted while the index does.
fn row(
    payee: &Payee,
    last: bool,
    selected: bool,
    focused: bool,
    props: &PayeesPageProps<'_>,
    cx: &App,
) -> impl IntoElement {
    let id = payee.id;
    let inverted = selected && focused;
    let hover = color::hover(cx);
    let (secondary, faint, missing, button_border): (Rgba, Rgba, Rgba, Rgba) = if inverted {
        (
            color::selection_muted(cx),
            color::selection_muted(cx),
            color::selection_accent_text(cx),
            color::selection_muted(cx),
        )
    } else {
        (
            color::muted(cx),
            color::faint_text(cx),
            color::accent_text(cx),
            color::border(cx),
        )
    };
    let category = payee
        .default_category
        .and_then(|category_id| props.categories.iter().find(|c| c.id == category_id));
    let rules = count(payee.aliases.len());
    let on_row_click = props.on_row_click.clone();
    let on_edit_click = props.on_edit_click.clone();
    let on_delete_click = props.on_delete_click.clone();

    div()
        .debug_selector(move || format!("settings-payees-row-{id}"))
        .id(SharedString::from(format!("settings-payees-row-{id}")))
        .cursor_pointer()
        .flex()
        .items_center()
        .px(px(14.0))
        .py(px(5.0))
        .text_size(px(12.5))
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::hairline(cx))
        })
        .when(selected && !focused, |this| this.bg(color::chrome(cx)))
        .when(inverted, |this| {
            this.bg(color::foreground(cx))
                .text_color(color::selection_text(cx))
        })
        .when(!selected, |this| this.hover(move |style| style.bg(hover)))
        // An inactive Payee stays listed, dimmed and tagged, rather than hidden.
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
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .child(payee.name.clone()),
                )
                .when(!payee.is_active, |this| {
                    this.child(
                        div()
                            .flex_none()
                            .px(px(6.0))
                            .py(px(1.0))
                            .border_1()
                            .border_color(button_border)
                            .text_size(px(10.0))
                            .text_color(secondary)
                            .child(crate::msg::desktop_payees_inactive()),
                    )
                }),
        )
        .child(div().w(CATEGORY_WIDTH).flex_none().truncate().map(|this| {
            match category {
                Some(category) => this.text_color(secondary).child(category.name.clone()),
                None => this
                    .text_color(missing)
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .child(crate::msg::desktop_payees_no_default_category()),
            }
        }))
        .child(
            div()
                .w(RULES_WIDTH)
                .flex_none()
                .text_color(faint)
                .child(crate::msg::desktop_payees_rule_count(rules)),
        )
        .child(
            div()
                .flex()
                .gap(px(4.0))
                .child(row_action_button(
                    SharedString::from(format!("settings-payees-edit-{id}")),
                    crate::msg::desktop_payees_row_edit(),
                    button_border,
                    Rc::new(move |window: &mut Window, cx: &mut App| on_edit_click(id, window, cx)),
                    cx,
                ))
                .child(row_action_button(
                    SharedString::from(format!("settings-payees-delete-{id}")),
                    crate::msg::desktop_payees_row_delete(),
                    button_border,
                    Rc::new(move |window: &mut Window, cx: &mut App| {
                        on_delete_click(id, window, cx)
                    }),
                    cx,
                )),
        )
}

/// `padding:2px 6px; font-size:11px; border:1px solid`. Stops the click reaching the row.
fn row_action_button(
    id: SharedString,
    label: String,
    border: Rgba,
    on_click: OnPlainClick,
    cx: &App,
) -> impl IntoElement {
    let hover = color::hover(cx);
    div()
        .debug_selector({
            let id = id.clone();
            move || id.to_string()
        })
        .id(id)
        .cursor_pointer()
        .py(px(2.0))
        .px(px(6.0))
        .border_1()
        .border_color(border)
        .text_size(px(11.0))
        .whitespace_nowrap()
        .hover(move |style| style.bg(hover))
        .on_click(move |_event, window, cx| {
            cx.stop_propagation();
            on_click(window, cx)
        })
        .child(label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_text_reads_the_count_and_the_missing_defaults() {
        crate::locale::init_for_tests();
        let payees = payees::default_payees();
        assert_eq!(
            scope_text(&payees),
            format!(
                "{} \u{b7} {} without default category",
                crate::msg::desktop_payees_count(count(payees::active_count(&payees))),
                payees::without_default_category_count(&payees)
            )
        );
    }

    #[test]
    fn sorted_by_name_ignores_case() {
        let payees = payees::default_payees();
        let names: Vec<String> = payees::sorted_by_name(&payees)
            .iter()
            .map(|payee| payee.name.to_lowercase())
            .collect();
        let mut expected = names.clone();
        expected.sort();
        assert_eq!(names, expected);
    }
}
