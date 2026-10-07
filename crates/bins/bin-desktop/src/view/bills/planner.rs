//! The **8b** Planner tab (`docs/ux/desktop/12-bills/README.md`): the Bill Plans themselves in one
//! bordered table (NAME / CATEGORY / ACCOUNT / PLANNED / RECURS / LEAD / ACTIVE / ACTIONS) and a
//! footnote, with the header's meta line. Not period-scoped, so no period nav.
//!
//! Rows come in `bills::planner_order` (active Plans by name, then inactive ones). A Plan with an
//! Attention Lead is tinted to draw the eye to LEAD; an inactive Plan is muted throughout; the
//! selected row inverts, as on the Schedule tab.

use gpui::{AnyElement, App, Rgba, SharedString, Window, div, prelude::*, px};

use lib_locale::format::upper;

use crate::{
    accounts::Account,
    bills::{AmountKind, BillPlan, Recurrence},
    categories::Category,
    theme::color,
    transactions::rows::EMPTY_CELL,
};

use super::OnRowClick;

pub struct PlannerProps<'a> {
    /// In `bills::planner_order`.
    pub plans: &'a [&'a BillPlan],
    pub inactive: usize,
    pub categories: &'a [Category],
    pub accounts: &'a [Account],
    /// The base Unit's code: an amount in it shows no Unit code.
    pub base_unit: Option<&'a str>,
    /// Index into `plans` of the selected row.
    pub selected: Option<usize>,
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

/// `10 bill plans · 1 inactive`, the inactive count bold; the clause is left out at zero.
pub fn meta_line(props: &PlannerProps<'_>, cx: &App) -> impl IntoElement {
    let count = i64::try_from(props.plans.len()).unwrap_or(i64::MAX);
    div()
        .flex()
        .flex_wrap()
        .items_baseline()
        .gap(px(4.0))
        .text_size(px(11.5))
        .text_color(color::faint_text(cx))
        .child(div().child(crate::msg::desktop_bills_planner_plans(count)))
        .when(props.inactive > 0, |this| {
            this.child(div().child("\u{b7}")).child(
                div().flex().children(
                    crate::msg::desktop_bills_planner_inactive(&props.inactive.to_string())
                        .into_iter()
                        .map(|segment| match segment.tag.as_deref() {
                            Some("strong") => div()
                                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                                .text_color(color::foreground(cx))
                                .child(segment.text)
                                .into_any_element(),
                            _ => div().child(segment.text).into_any_element(),
                        }),
                ),
            )
        })
}

pub fn recurrence_label(recurrence: Recurrence) -> String {
    match recurrence {
        Recurrence::Weekly => crate::msg::desktop_bills_recurrence_weekly(),
        Recurrence::Fortnightly => crate::msg::desktop_bills_recurrence_fortnightly(),
        Recurrence::Monthly => crate::msg::desktop_bills_recurrence_monthly(),
        Recurrence::Quarterly => crate::msg::desktop_bills_recurrence_quarterly(),
        Recurrence::Annually => crate::msg::desktop_bills_recurrence_annually(),
        Recurrence::OneShot => crate::msg::desktop_bills_recurrence_one_shot(),
    }
}

/// PLANNED's text: the Plan's figure, the Unit code appended off the base Unit, `~`-prefixed when
/// Estimated.
fn planned_text(
    amount: &lib_core::Money,
    unit: &str,
    kind: AmountKind,
    base_unit: Option<&str>,
) -> String {
    let mut text = crate::format::amount(amount).1;
    if Some(unit) != base_unit {
        text = format!("{text} {unit}");
    }
    if kind == AmountKind::Estimated {
        format!("~{text}")
    } else {
        text
    }
}

/// LEAD's text: `3d`, or `—` when unset (Overdue-only).
fn lead_text(attention_lead: Option<u32>) -> String {
    attention_lead.map_or_else(
        || EMPTY_CELL.to_string(),
        |days| crate::msg::desktop_bills_lead_days(&days.to_string()),
    )
}

pub fn render(props: &PlannerProps<'_>, cx: &App) -> AnyElement {
    if props.plans.is_empty() {
        return div()
            .text_color(color::muted(cx))
            .child(crate::msg::desktop_bills_planner_empty())
            .into_any_element();
    }
    let last = props.plans.len() - 1;
    div()
        .flex()
        .flex_col()
        .child(
            div()
                .flex()
                .flex_col()
                .border_1()
                .border_color(color::border(cx))
                .child(table_header(cx))
                .children(props.plans.iter().enumerate().map(|(index, plan)| {
                    render_row(
                        index,
                        plan,
                        index == last,
                        props.selected == Some(index),
                        props,
                        cx,
                    )
                })),
        )
        .child(
            div()
                .mt(px(10.0))
                .text_size(px(11.0))
                .text_color(color::faint_text(cx))
                .child(crate::msg::desktop_bills_planner_footnote()),
        )
        .into_any_element()
}

fn table_header(cx: &App) -> impl IntoElement {
    let cell = |width, label: String| div().w(width).flex_none().child(upper(&label));
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
                .child(upper(&crate::msg::desktop_bills_column_name())),
        )
        .child(cell(
            CATEGORY_WIDTH,
            crate::msg::desktop_bills_column_category(),
        ))
        .child(cell(
            ACCOUNT_WIDTH,
            crate::msg::desktop_bills_column_account(),
        ))
        .child(right(
            PLANNED_WIDTH,
            crate::msg::desktop_bills_column_planned(),
        ))
        .child(
            div()
                .w(RECURS_WIDTH)
                .flex_none()
                // The handoff runs RECURS flush against PLANNED; a gap keeps them apart.
                .pl(px(16.0))
                .child(upper(&crate::msg::desktop_bills_column_recurs())),
        )
        .child(cell(LEAD_WIDTH, crate::msg::desktop_bills_column_lead()))
        .child(cell(
            ACTIVE_WIDTH,
            crate::msg::desktop_bills_column_active(),
        ))
        .child(right(ACTIONS_WIDTH, lib_locale::msg::column_actions()))
}

fn render_row(
    index: usize,
    plan: &BillPlan,
    last: bool,
    selected: bool,
    props: &PlannerProps<'_>,
    cx: &App,
) -> AnyElement {
    let inactive = !plan.is_active;
    let lead = plan.attention_lead.is_some() && !inactive;
    let (text, secondary, accent, button_border): (Rgba, Rgba, Rgba, Rgba) = if selected {
        (
            color::selection_text(cx),
            color::selection_muted(cx),
            color::selection_accent_text(cx),
            color::selection_muted(cx),
        )
    } else if inactive {
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
    let category = props
        .categories
        .iter()
        .find(|category| category.id == plan.category_id)
        .map_or_else(|| EMPTY_CELL.to_string(), |category| category.name.clone());
    let account = props
        .accounts
        .iter()
        .find(|account| account.id == plan.account_id)
        .map_or_else(|| EMPTY_CELL.to_string(), |account| account.name.clone());
    let active = if plan.is_active {
        crate::msg::desktop_bills_active_yes()
    } else {
        crate::msg::desktop_bills_active_no()
    };
    let on_row_click = props.on_row_click.clone();
    let on_edit_click = props.on_edit_click.clone();
    let edit_hover = color::hover(cx);

    div()
        .id(SharedString::from(format!("bills-plan-row-{}", plan.id)))
        .debug_selector({
            let id = plan.id;
            move || format!("bills-plan-row-{id}")
        })
        .cursor_pointer()
        .flex()
        .items_center()
        .px(px(16.0))
        .py(px(10.0))
        .text_color(text)
        .when(!last, |this| {
            this.border_b(px(1.0)).border_color(color::hairline(cx))
        })
        .when(selected, |this| this.bg(color::selection_background(cx)))
        .when(!selected && lead, |this| this.bg(color::accent_tint(cx)))
        .when(!selected, |this| this.hover(move |style| style.bg(hover)))
        .on_click(move |_event, window, cx| on_row_click(index, window, cx))
        .child(
            div()
                .flex_1()
                .min_w(NAME_MIN_WIDTH)
                .truncate()
                .when(selected, |this| {
                    this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                })
                .child(plan.name.clone()),
        )
        .child(
            div()
                .w(CATEGORY_WIDTH)
                .flex_none()
                .truncate()
                .text_color(secondary)
                .child(category),
        )
        .child(
            div()
                .w(ACCOUNT_WIDTH)
                .flex_none()
                .truncate()
                .text_color(secondary)
                .child(account),
        )
        .child(
            div()
                .w(PLANNED_WIDTH)
                .flex_none()
                .flex()
                .justify_end()
                .whitespace_nowrap()
                .child(planned_text(
                    &plan.planned_amount,
                    &plan.unit,
                    plan.amount_kind,
                    props.base_unit,
                )),
        )
        .child(
            div()
                .w(RECURS_WIDTH)
                .flex_none()
                .pl(px(16.0))
                .whitespace_nowrap()
                .text_color(secondary)
                .child(recurrence_label(plan.recurrence)),
        )
        .child(
            div()
                .w(LEAD_WIDTH)
                .flex_none()
                .whitespace_nowrap()
                .when(lead, |this| {
                    this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_color(accent)
                })
                .when(!lead, |this| this.text_color(secondary))
                .child(lead_text(plan.attention_lead)),
        )
        .child(
            div()
                .w(ACTIVE_WIDTH)
                .flex_none()
                .text_color(secondary)
                .child(active),
        )
        .child(
            div()
                .w(ACTIONS_WIDTH)
                .flex_none()
                .flex()
                .justify_end()
                .child(
                    div()
                        .id(SharedString::from(format!("bills-plan-edit-{}", plan.id)))
                        .debug_selector({
                            let id = plan.id;
                            move || format!("bills-plan-edit-{id}")
                        })
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
                        .child(crate::msg::desktop_bills_row_edit()),
                ),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planned_marks_estimates_and_foreign_units() {
        crate::locale::init_for_tests();
        let amount = lib_core::Money(bigdecimal::BigDecimal::new(8999.into(), 2));
        let planned = |kind, base| planned_text(&amount, "aud", kind, base);
        assert_eq!(planned(AmountKind::Fixed, Some("aud")), "89.99");
        assert_eq!(planned(AmountKind::Estimated, Some("aud")), "~89.99");
        assert_eq!(planned(AmountKind::Estimated, Some("usd")), "~89.99 aud");
    }

    #[test]
    fn lead_shows_days_or_a_dash() {
        crate::locale::init_for_tests();
        assert_eq!(lead_text(Some(3)), "3d");
        assert_eq!(lead_text(None), EMPTY_CELL);
    }
}
