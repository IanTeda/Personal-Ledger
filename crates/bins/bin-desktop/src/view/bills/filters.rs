//! The Schedule tab's filter row and stat callout (`docs/ux/desktop/Bills/README.md`'s 8a, carrying
//! over 8f's History pieces in #381): status chips, then the Bill, Category and Account selects,
//! and the Last paid / Average / Same period last year figures while scoped to one Bill Plan.
//!
//! The filters and figures come from `bill_history`; the selects are the dialogs' shared dropdown,
//! opening inline beneath the filter row.

use std::rc::Rc;

use gpui::{AnyElement, App, SharedString, Window, div, prelude::*, px};

use lib_locale::format::format_month_day;

use crate::{
    bill_history::{BillFilters, PlanStats, STATUS_CHIPS, SamePeriod},
    bills::{BillPlan, BillStatus, Recurrence},
    select::SelectState,
    theme::color,
    transaction_rows::EMPTY_CELL,
    view::accounts::select_field::{self, SelectFieldProps},
};

use super::{OnRowClick, schedule::with_unit};

/// Called with a select's field and the chosen option's index.
pub type OnOptionClick = Rc<dyn Fn(FilterField, usize, &mut Window, &mut App)>;
pub type OnFieldClick = Rc<dyn Fn(FilterField, &mut Window, &mut App)>;

/// The filter row's selects, in `f`'s order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterField {
    Plan,
    Category,
    Account,
}

impl FilterField {
    pub const ORDER: [FilterField; 3] = [Self::Plan, Self::Category, Self::Account];

    fn id(self) -> &'static str {
        match self {
            Self::Plan => "bills-filter-plan",
            Self::Category => "bills-filter-category",
            Self::Account => "bills-filter-account",
        }
    }

    fn label(self) -> String {
        match self {
            Self::Plan => crate::msg::desktop_bills_column_bill(),
            Self::Category => crate::msg::desktop_bills_column_category(),
            Self::Account => crate::msg::desktop_bills_column_account(),
        }
    }
}

/// One select as `Shell` hands it over: its options and state (the focused one's live state, the
/// rest a closed state on their current value).
pub struct FilterSelect {
    pub field: FilterField,
    pub options: Vec<String>,
    pub state: SelectState,
    pub focused: bool,
}

pub struct FilterProps<'a> {
    pub filters: &'a BillFilters,
    pub selects: Vec<FilterSelect>,
    /// The scoped Bill Plan and its figures, when the Bill select names one.
    pub stats: Option<(&'a BillPlan, PlanStats)>,
    /// The base Unit's code: an amount in it shows no Unit code.
    pub base_unit: Option<&'a str>,
    /// Called with the chip's index in `STATUS_CHIPS`.
    pub on_chip_click: OnRowClick,
    pub on_field_click: OnFieldClick,
    pub on_option_click: OnOptionClick,
}

const SELECT_WIDTH: gpui::Pixels = px(180.0);

pub fn status_label(status: BillStatus) -> String {
    match status {
        BillStatus::Paid => crate::msg::desktop_bills_filter_status_paid(),
        BillStatus::Skipped => crate::msg::desktop_bills_filter_status_skipped(),
        BillStatus::Overdue => crate::msg::desktop_bills_filter_status_overdue(),
        BillStatus::Due => crate::msg::desktop_bills_filter_status_due(),
        BillStatus::Upcoming => crate::msg::desktop_bills_filter_status_upcoming(),
    }
}

/// The filter row, then the stat callout while scoped to one Bill Plan.
pub fn render(props: &FilterProps<'_>, cx: &App) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .child(filter_row(props, cx))
        .when_some(props.stats.as_ref(), |this, (plan, stats)| {
            this.child(stat_callout(plan, stats, props.base_unit, cx))
        })
        .into_any_element()
}

fn filter_row(props: &FilterProps<'_>, cx: &App) -> impl IntoElement {
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
                        accent: field == FilterField::Plan && props.filters.plan_id.is_some(),
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
    props: &FilterProps<'_>,
    cx: &App,
) -> AnyElement {
    let on_click = props.on_chip_click.clone();
    let hover = color::hover(cx);
    let label = status_label(status);
    div()
        .id(SharedString::from(format!("bills-filter-chip-{index}")))
        .debug_selector(move || format!("bills-filter-chip-{index}"))
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
    let money = |amount: &lib_core::Money| {
        with_unit(crate::format::amount(amount).1, &plan.unit, base_unit)
    };
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
