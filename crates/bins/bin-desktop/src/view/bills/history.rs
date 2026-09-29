//! The **8f** History tab (`docs/ux/desktop/Bills/README.md`): every Bill Schedule entry that
//! isn't superseded, read-only. A filter row (status chips, then the Bill, Category, Account and
//! date-range selects), the stat callout while scoped to one Bill Plan, the table (BILL / DUE /
//! STATUS / PLANNED / ACTUAL / PAID) and a footnote.
//!
//! The rows and figures come from `bill_history`; the selects are the dialogs' shared dropdown,
//! opening inline beneath the filter row. A Skipped row shows its planned figure but `—` for
//! ACTUAL and PAID.

use std::rc::Rc;

use gpui::{AnyElement, App, Rgba, SharedString, Window, div, prelude::*, px};

use lib_locale::format::{format_month_day, upper};

use crate::{
    bill_history::{DateRange, HistoryFilters, HistoryRow, PlanStats, STATUS_CHIPS, SamePeriod},
    bills::{self, BillPlan, BillScheduleEntry, BillStatus, Recurrence},
    select::SelectState,
    theme::color,
    transaction_rows::EMPTY_CELL,
    transactions::Transaction,
    view::accounts::select_field::{self, SelectFieldProps},
};

use super::OnRowClick;

/// Called with a select's field and the chosen option's index.
pub type OnOptionClick = Rc<dyn Fn(HistoryField, usize, &mut Window, &mut App)>;
pub type OnFieldClick = Rc<dyn Fn(HistoryField, &mut Window, &mut App)>;

/// The filter row's selects, in `f`'s order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryField {
    Plan,
    Category,
    Account,
    Range,
}

impl HistoryField {
    pub const ORDER: [HistoryField; 4] = [Self::Plan, Self::Category, Self::Account, Self::Range];

    fn id(self) -> &'static str {
        match self {
            Self::Plan => "bills-history-plan",
            Self::Category => "bills-history-category",
            Self::Account => "bills-history-account",
            Self::Range => "bills-history-range",
        }
    }

    fn label(self) -> String {
        match self {
            Self::Plan => crate::msg::desktop_bills_column_bill(),
            Self::Category => crate::msg::desktop_bills_column_category(),
            Self::Account => crate::msg::desktop_bills_column_account(),
            Self::Range => crate::msg::desktop_bills_history_range(),
        }
    }
}

/// One select as `Shell` hands it over: its options and state (the focused one's live state, the
/// rest a closed state on their current value).
pub struct HistorySelect {
    pub field: HistoryField,
    pub options: Vec<String>,
    pub state: SelectState,
    pub focused: bool,
}

pub struct HistoryProps<'a> {
    pub rows: &'a [HistoryRow],
    /// Every non-superseded entry, before filtering.
    pub total: usize,
    pub filters: &'a HistoryFilters,
    pub selects: Vec<HistorySelect>,
    /// The scoped Bill Plan and its figures, when the Bill select names one.
    pub stats: Option<(&'a BillPlan, PlanStats)>,
    pub plans: &'a [BillPlan],
    pub entries: &'a [BillScheduleEntry],
    pub transactions: &'a [Transaction],
    /// The base Unit's code: an amount in it shows no Unit code.
    pub base_unit: Option<&'a str>,
    /// Index into `rows` of the selected row.
    pub selected: Option<usize>,
    pub on_row_click: OnRowClick,
    /// Called with the chip's index in `STATUS_CHIPS`.
    pub on_chip_click: OnRowClick,
    pub on_field_click: OnFieldClick,
    pub on_option_click: OnOptionClick,
}

const BILL_MIN_WIDTH: gpui::Pixels = px(160.0);
const DUE_WIDTH: gpui::Pixels = px(90.0);
const STATUS_WIDTH: gpui::Pixels = px(100.0);
const AMOUNT_WIDTH: gpui::Pixels = px(100.0);
const PAID_WIDTH: gpui::Pixels = px(90.0);
const SELECT_WIDTH: gpui::Pixels = px(180.0);

pub fn status_label(status: BillStatus) -> String {
    match status {
        BillStatus::Paid => crate::msg::desktop_bills_history_status_paid(),
        BillStatus::Skipped => crate::msg::desktop_bills_history_status_skipped(),
        BillStatus::Overdue => crate::msg::desktop_bills_history_status_overdue(),
        BillStatus::Due => crate::msg::desktop_bills_history_status_due(),
        BillStatus::Upcoming => crate::msg::desktop_bills_history_status_upcoming(),
    }
}

pub fn range_label(range: DateRange) -> String {
    match range {
        DateRange::ThisFinancialYear => crate::msg::desktop_bills_range_this_financial_year(),
        DateRange::LastFinancialYear => crate::msg::desktop_bills_range_last_financial_year(),
        DateRange::Last12Months => crate::msg::desktop_bills_range_last_12_months(),
        DateRange::ThisCalendarYear => crate::msg::desktop_bills_range_this_calendar_year(),
        DateRange::AllTime => crate::msg::desktop_bills_range_all_time(),
    }
}

/// An amount with the Unit code appended off the base Unit.
fn money_text(amount: &lib_core::Money, unit: &str, base_unit: Option<&str>) -> String {
    let text = crate::format::amount(amount).1;
    if Some(unit) == base_unit {
        text
    } else {
        format!("{text} {unit}")
    }
}

/// `Every schedule row ever generated`, naming the scoped Bill Plan when there is one.
pub fn meta_line(props: &HistoryProps<'_>, cx: &App) -> impl IntoElement {
    let text = match &props.stats {
        Some((plan, _)) => crate::msg::desktop_bills_history_meta_scoped(&plan.name),
        None => crate::msg::desktop_bills_history_meta(),
    };
    div()
        .text_size(px(11.5))
        .text_color(color::faint_text(cx))
        .child(text)
}

pub fn render(props: &HistoryProps<'_>, cx: &App) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .child(filter_row(props, cx))
        .when_some(props.stats.as_ref(), |this, (plan, stats)| {
            this.child(stat_callout(plan, stats, props.base_unit, cx))
        })
        .child(table(props, cx))
        .child(
            div()
                .mt(px(10.0))
                .text_size(px(11.0))
                .text_color(color::faint_text(cx))
                .child(crate::msg::desktop_bills_history_footnote()),
        )
        .into_any_element()
}

fn filter_row(props: &HistoryProps<'_>, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .flex_wrap()
        .items_end()
        .gap(px(8.0))
        .mb(px(20.0))
        .children(
            STATUS_CHIPS.iter().enumerate().map(|(index, status)| {
                chip(index, *status, props.filters.shows(*status), props, cx)
            }),
        )
        .child(
            div()
                .w(px(1.0))
                .h(px(24.0))
                .mx(px(4.0))
                .mb(px(6.0))
                .bg(color::border(cx)),
        )
        .children(props.selects.iter().map(|select| {
            let field = select.field;
            let on_field_click = props.on_field_click.clone();
            let on_option_click = props.on_option_click.clone();
            div()
                .w(SELECT_WIDTH)
                .flex_none()
                .child(select_field::render(
                    SelectFieldProps {
                        id: field.id(),
                        label: field.label().into(),
                        options: &select.options,
                        state: &select.state,
                        focused: select.focused,
                        accent: field == HistoryField::Plan && props.filters.plan_id.is_some(),
                        read_only: None,
                        on_field_click: Rc::new(move |window, cx| {
                            on_field_click(field, window, cx)
                        }),
                        on_option_click: Rc::new(move |index, window, cx| {
                            on_option_click(field, index, window, cx)
                        }),
                    },
                    cx,
                ))
        }))
}

/// A status chip: dark and `✓`-marked while on, outlined while off. The digit is its `1`–`5` key.
fn chip(
    index: usize,
    status: BillStatus,
    on: bool,
    props: &HistoryProps<'_>,
    cx: &App,
) -> AnyElement {
    let on_click = props.on_chip_click.clone();
    let hover = color::hover(cx);
    let label = status_label(status);
    div()
        .id(SharedString::from(format!("bills-history-chip-{index}")))
        .cursor_pointer()
        .flex_none()
        .py(px(8.0))
        .px(px(12.0))
        .border_1()
        .text_size(px(12.0))
        .when(on, |this| {
            this.bg(color::selection_background(cx))
                .border_color(color::selection_background(cx))
                .text_color(color::selection_text(cx))
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .child(format!("\u{2713} {label}"))
        })
        .when(!on, |this| {
            this.border_color(color::border(cx))
                .text_color(color::muted(cx))
                .hover(move |style| style.bg(hover))
                .child(label)
        })
        .on_click(move |_event, window, cx| on_click(index, window, cx))
        .into_any_element()
}

/// The three figures for the scoped Bill Plan; `—` where there is nothing to show.
fn stat_callout(
    plan: &BillPlan,
    stats: &PlanStats,
    base_unit: Option<&str>,
    cx: &App,
) -> impl IntoElement {
    let money = |amount: &lib_core::Money| money_text(amount, &plan.unit, base_unit);
    let stat = |label: String, figure: Option<String>, note: Option<String>| {
        div()
            .flex()
            .items_baseline()
            .gap(px(4.0))
            .child(div().text_color(color::muted(cx)).child(label))
            .child(
                div()
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .text_color(color::foreground(cx))
                    .child(figure.unwrap_or_else(|| EMPTY_CELL.to_string())),
            )
            .when_some(note, |this, note| {
                this.child(
                    div()
                        .text_color(color::faint_text(cx))
                        .child(format!("\u{b7} {note}")),
                )
            })
    };
    let last_paid = stat(
        crate::msg::desktop_bills_history_last_paid(),
        stats.last_paid.as_ref().map(|(amount, _)| money(amount)),
        stats
            .last_paid
            .as_ref()
            .map(|(_, date)| format_month_day(*date)),
    );
    let year = |y: i32| format!("{:02}", y.rem_euclid(100));
    let average = stat(
        crate::msg::desktop_bills_history_average(
            &year(stats.average_year),
            &year(stats.average_year + 1),
        ),
        stats.average.as_ref().map(money),
        (stats.average.is_none() && !stats.never_paid)
            .then(crate::msg::desktop_bills_history_no_payments_last_fy),
    );
    let same_period = match &stats.same_period {
        SamePeriod::Hidden => None,
        SamePeriod::None => Some(stat(same_period_label(plan.recurrence, 0), None, None)),
        SamePeriod::Payment(amount) => Some(stat(
            same_period_label(plan.recurrence, 1),
            Some(money(amount)),
            None,
        )),
        SamePeriod::Month { total, count } => Some(stat(
            same_period_label(plan.recurrence, *count),
            Some(money(total)),
            None,
        )),
    };
    div()
        .flex()
        .flex_wrap()
        .items_baseline()
        .gap(px(24.0))
        .mb(px(20.0))
        .py(px(14.0))
        .px(px(16.0))
        .bg(color::chrome(cx))
        .border_l(px(2.0))
        .border_color(color::foreground(cx))
        .text_size(px(13.0))
        .child(last_paid)
        .child(average)
        .children(same_period)
        .when(stats.never_paid, |this| {
            this.child(
                div()
                    .text_color(color::faint_text(cx))
                    .child(crate::msg::desktop_bills_history_no_payments_yet()),
            )
        })
}

fn same_period_label(recurrence: Recurrence, count: usize) -> String {
    match recurrence {
        Recurrence::Weekly | Recurrence::Fortnightly => {
            crate::msg::desktop_bills_history_same_month_payments(
                i64::try_from(count).unwrap_or(i64::MAX),
            )
        }
        Recurrence::Annually => crate::msg::desktop_bills_history_last_year(),
        Recurrence::Monthly | Recurrence::Quarterly | Recurrence::OneShot => {
            crate::msg::desktop_bills_history_same_month()
        }
    }
}

fn table(props: &HistoryProps<'_>, cx: &App) -> AnyElement {
    if props.rows.is_empty() {
        return div()
            .text_color(color::muted(cx))
            .child(crate::msg::desktop_bills_history_empty())
            .into_any_element();
    }
    let last = props.rows.len() - 1;
    div()
        .flex()
        .flex_col()
        .border_1()
        .border_color(color::border(cx))
        .child(table_header(cx))
        .children(props.rows.iter().enumerate().filter_map(|(index, row)| {
            let plan = bills::get(props.plans, row.id.plan_id)?;
            Some(render_row(
                index,
                row,
                plan,
                index == last,
                props.selected == Some(index),
                props,
                cx,
            ))
        }))
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
                .min_w(BILL_MIN_WIDTH)
                .child(upper(&crate::msg::desktop_bills_column_bill())),
        )
        .child(cell(DUE_WIDTH, crate::msg::desktop_bills_column_due()))
        .child(cell(
            STATUS_WIDTH,
            crate::msg::desktop_bills_column_status(),
        ))
        .child(right(
            AMOUNT_WIDTH,
            crate::msg::desktop_bills_column_planned(),
        ))
        .child(right(
            AMOUNT_WIDTH,
            crate::msg::desktop_bills_column_actual(),
        ))
        .child(
            div()
                .w(PAID_WIDTH)
                .flex_none()
                // The handoff runs PAID flush against ACTUAL; a gap keeps them apart.
                .pl(px(16.0))
                .child(upper(&crate::msg::desktop_bills_column_paid())),
        )
}

fn render_row(
    index: usize,
    row: &HistoryRow,
    plan: &BillPlan,
    last: bool,
    selected: bool,
    props: &HistoryProps<'_>,
    cx: &App,
) -> AnyElement {
    let quiet = matches!(row.status, BillStatus::Skipped | BillStatus::Upcoming);
    let (text, secondary, accent): (Rgba, Rgba, Rgba) = if selected {
        (
            color::selection_text(cx),
            color::selection_muted(cx),
            color::selection_accent_text(cx),
        )
    } else if quiet {
        let faint = color::faint_text(cx);
        (faint, faint, faint)
    } else {
        (
            color::foreground(cx),
            color::muted(cx),
            color::accent_text(cx),
        )
    };
    let hover = color::hover(cx);
    let entry = bills::entry(props.entries, row.id);
    // PLANNED is the Plan's figure (a Skipped row's snapshot); ACTUAL the Matched Split's.
    let planned = match entry.map(|e| &e.resolution) {
        Some(bills::Resolution::Skipped { planned }) => planned.clone(),
        _ => plan.planned_amount.clone(),
    };
    let actual = (row.status == BillStatus::Paid)
        .then(|| entry.and_then(|e| bills::amount(e, props.plans, props.transactions)))
        .flatten();
    let paid_on = entry.and_then(|e| bills::paid_on(e, props.transactions));
    let on_row_click = props.on_row_click.clone();

    div()
        .id(SharedString::from(format!(
            "bills-history-row-{}-{}",
            row.id.plan_id, row.id.due
        )))
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
        .when(!selected, |this| this.hover(move |style| style.bg(hover)))
        .on_click(move |_event, window, cx| on_row_click(index, window, cx))
        .child(
            div()
                .flex_1()
                .min_w(BILL_MIN_WIDTH)
                .truncate()
                .when(selected, |this| {
                    this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                })
                .child(plan.name.clone()),
        )
        .child(
            div()
                .w(DUE_WIDTH)
                .flex_none()
                .whitespace_nowrap()
                .text_color(secondary)
                .child(format_month_day(row.id.due)),
        )
        .child(
            div()
                .w(STATUS_WIDTH)
                .flex_none()
                .whitespace_nowrap()
                .when(row.status == BillStatus::Overdue, |this| {
                    this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_color(accent)
                })
                .child(status_label(row.status)),
        )
        .child(
            div()
                .w(AMOUNT_WIDTH)
                .flex_none()
                .flex()
                .justify_end()
                .whitespace_nowrap()
                .child(money_text(&planned, &plan.unit, props.base_unit)),
        )
        .child(
            div()
                .w(AMOUNT_WIDTH)
                .flex_none()
                .flex()
                .justify_end()
                .whitespace_nowrap()
                .when(selected && actual.is_some(), |this| {
                    this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                })
                .child(actual.map_or_else(
                    || EMPTY_CELL.to_string(),
                    |amount| money_text(&amount, &plan.unit, props.base_unit),
                )),
        )
        .child(
            div()
                .w(PAID_WIDTH)
                .flex_none()
                .pl(px(16.0))
                .whitespace_nowrap()
                .text_color(secondary)
                .child(paid_on.map_or_else(|| EMPTY_CELL.to_string(), format_month_day)),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn money_shows_the_unit_off_the_base() {
        crate::locale::init_for_tests();
        let amount = lib_core::Money(bigdecimal::BigDecimal::new(7_999.into(), 2));
        assert_eq!(money_text(&amount, "aud", Some("aud")), "79.99");
        assert_eq!(money_text(&amount, "aud", Some("usd")), "79.99 aud");
    }
}
