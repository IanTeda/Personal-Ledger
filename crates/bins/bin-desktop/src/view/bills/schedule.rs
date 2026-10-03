//! The **8a** Schedule tab (`docs/ux/desktop/12-bills/README.md`), which absorbed 8f's History tab
//! (#381): the filter row and stat callout (`filters`), then the rows in one bordered table (BILL /
//! ACCOUNT / PLANNED / ACTUAL / DUE / PAID / STATUS / ACTIONS) and a footnote, with the header's
//! meta line.
//!
//! Rows come from `bills::schedule_rows` for the viewed month (or All), through the filters: the
//! month's entries, Overdue and Due entries carried in from other months, and computed previews
//! past the generation horizon, unresolved first. A row's status, Needs Attention flag and actions
//! are all derived there; this module only draws them. The selected row inverts, as in the
//! handoff. A Skipped row keeps its planned figure but shows `—` for ACTUAL and PAID.

use gpui::{AnyElement, App, Rgba, SharedString, Window, div, prelude::*, px};

use lib_locale::format::{format_month_day, upper};

use crate::{
    accounts::Account,
    bills::{
        self, AmountKind, BillPlan, BillScheduleEntry, BillStatus, PeriodSummary, ScheduleRow,
    },
    settings::StatusGlyphs,
    theme::color,
    transaction_query::Total,
    transaction_rows::EMPTY_CELL,
    transactions::Transaction,
};

use super::{OnPlainClick, OnRowClick, filters};

pub struct ScheduleProps<'a> {
    /// The filtered rows.
    pub rows: &'a [ScheduleRow],
    /// `rows`' count before filtering: the `M` in the status line's "N of M".
    pub total: usize,
    /// Viewing All rather than one month.
    pub all: bool,
    pub filters: filters::FilterProps<'a>,
    pub summary: &'a PeriodSummary,
    pub plans: &'a [BillPlan],
    pub entries: &'a [BillScheduleEntry],
    pub accounts: &'a [Account],
    pub transactions: &'a [Transaction],
    /// The base Unit's code: an amount in it shows no Unit code.
    pub base_unit: Option<&'a str>,
    pub glyphs: StatusGlyphs,
    /// Index into `rows` of the selected row.
    pub selected: Option<usize>,
    pub on_row_click: OnRowClick,
    pub on_pay_click: OnRowClick,
    pub on_skip_click: OnRowClick,
    pub on_view_transaction_click: OnRowClick,
}

const BILL_MIN_WIDTH: gpui::Pixels = px(160.0);
const ACCOUNT_WIDTH: gpui::Pixels = px(120.0);
const PLANNED_WIDTH: gpui::Pixels = px(110.0);
const ACTUAL_WIDTH: gpui::Pixels = px(100.0);
const DUE_WIDTH: gpui::Pixels = px(90.0);
const PAID_WIDTH: gpui::Pixels = px(90.0);
const STATUS_WIDTH: gpui::Pixels = px(140.0);
const ACTIONS_WIDTH: gpui::Pixels = px(200.0);

/// What a row's ACTIONS cell holds, by status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowActions {
    PayOrSkip,
    ViewTransaction,
    /// Skipped: left out of the Bill Plan's history figures.
    Excluded,
    /// Upcoming, or a preview past the horizon: nothing to do yet.
    NotActionable,
}

fn row_actions(row: &ScheduleRow) -> RowActions {
    match row.status {
        BillStatus::Paid => RowActions::ViewTransaction,
        BillStatus::Skipped => RowActions::Excluded,
        _ if row.is_actionable() => RowActions::PayOrSkip,
        _ => RowActions::NotActionable,
    }
}

/// `9 schedule entries this period · 2 overdue · 3 due · 1 paid · 842.50 planned`, the Overdue and
/// Due counts bold (Overdue in the accent text colour); a zero count's clause is left out.
pub fn meta_line(props: &ScheduleProps<'_>, cx: &App) -> impl IntoElement {
    let count = |n: usize| i64::try_from(n).unwrap_or(i64::MAX);
    let summary = props.summary;
    let rich = |segments: Vec<lib_locale::Segment>, strong: Rgba| {
        div()
            .flex()
            .children(segments.into_iter().map(move |segment| {
                match segment.tag.as_deref() {
                    Some("strong") => div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_color(strong)
                        .child(segment.text)
                        .into_any_element(),
                    _ => div().child(segment.text).into_any_element(),
                }
            }))
    };
    let mut clauses: Vec<AnyElement> = vec![
        div()
            .child(if props.all {
                crate::msg::desktop_bills_schedule_entries_all(count(props.rows.len()))
            } else {
                crate::msg::desktop_bills_schedule_entries(count(props.rows.len()))
            })
            .into_any_element(),
    ];
    if summary.overdue > 0 {
        clauses.push(
            rich(
                crate::msg::desktop_bills_schedule_overdue(&summary.overdue.to_string()),
                color::accent_text(cx),
            )
            .into_any_element(),
        );
    }
    if summary.due > 0 {
        clauses.push(
            rich(
                crate::msg::desktop_bills_schedule_due(&summary.due.to_string()),
                color::foreground(cx),
            )
            .into_any_element(),
        );
    }
    if summary.paid > 0 {
        clauses.push(
            div()
                .child(crate::msg::desktop_bills_schedule_paid(
                    &summary.paid.to_string(),
                ))
                .into_any_element(),
        );
    }
    if let Some(total) = total_text(&summary.planned, props.base_unit) {
        clauses.push(
            div()
                .child(crate::msg::desktop_bills_schedule_planned(&total))
                .into_any_element(),
        );
    }
    let last = clauses.len().saturating_sub(1);
    div()
        .flex()
        .flex_wrap()
        .items_baseline()
        .gap(px(4.0))
        .text_size(px(11.5))
        .text_color(color::faint_text(cx))
        .children(
            clauses
                .into_iter()
                .enumerate()
                .flat_map(move |(index, clause)| {
                    let separator =
                        (index < last).then(|| div().child("\u{b7}").into_any_element());
                    std::iter::once(clause).chain(separator)
                }),
        )
}

/// The planned total, the Unit code appended when it isn't the base Unit; `None` for an empty
/// period.
fn total_text(total: &Total, base_unit: Option<&str>) -> Option<String> {
    match total {
        Total::Empty => None,
        Total::Mixed => Some(crate::msg::desktop_bills_total_mixed()),
        Total::Single { unit, amount } => {
            Some(with_unit(crate::format::amount(amount).1, unit, base_unit))
        }
    }
}

/// An amount's text, naming its Unit only when that isn't the base Unit.
pub(crate) fn with_unit(text: String, unit: &str, base_unit: Option<&str>) -> String {
    if Some(unit) == base_unit {
        text
    } else {
        format!("{text} {unit}")
    }
}

/// PLANNED's text: the Plan's Planned Amount (a Skipped row's snapshot), `~`-prefixed for an
/// Estimated Plan until it is Paid.
fn planned_text(row: &ScheduleRow, plan: &BillPlan, props: &ScheduleProps<'_>) -> String {
    let amount = match bills::entry(props.entries, row.id).map(|e| &e.resolution) {
        Some(bills::Resolution::Skipped { planned }) => planned.clone(),
        _ => plan.planned_amount.clone(),
    };
    let text = with_unit(
        crate::format::amount(&amount).1,
        &plan.unit,
        props.base_unit,
    );
    if plan.amount_kind == AmountKind::Estimated && row.status != BillStatus::Paid {
        format!("~{text}")
    } else {
        text
    }
}

/// ACTUAL's text: a Paid row's Matched amount, `—` otherwise.
fn actual_text(row: &ScheduleRow, plan: &BillPlan, props: &ScheduleProps<'_>) -> String {
    (row.status == BillStatus::Paid)
        .then(|| bills::entry(props.entries, row.id))
        .flatten()
        .and_then(|e| bills::amount(e, props.plans, props.transactions))
        .map_or_else(
            || EMPTY_CELL.to_string(),
            |amount| {
                with_unit(
                    crate::format::amount(&amount).1,
                    &plan.unit,
                    props.base_unit,
                )
            },
        )
}

/// PAID's text: the Matched Transaction's date, `—` otherwise.
fn paid_text(row: &ScheduleRow, props: &ScheduleProps<'_>) -> String {
    bills::entry(props.entries, row.id)
        .and_then(|e| bills::paid_on(e, props.transactions))
        .map_or_else(|| EMPTY_CELL.to_string(), format_month_day)
}

pub fn render(props: &ScheduleProps<'_>, cx: &App) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .child(filters::render(&props.filters, cx))
        .child(table(props, cx))
        .child(
            div()
                .mt(px(10.0))
                .text_size(px(11.0))
                .text_color(color::faint_text(cx))
                .child(crate::msg::desktop_bills_schedule_footnote()),
        )
        .when(props.filters.stats.is_some(), |this| {
            this.child(
                div()
                    .mt(px(6.0))
                    .text_size(px(11.0))
                    .text_color(color::faint_text(cx))
                    .child(crate::msg::desktop_bills_history_footnote()),
            )
        })
        .into_any_element()
}

fn table(props: &ScheduleProps<'_>, cx: &App) -> AnyElement {
    if props.rows.is_empty() {
        let empty = if props.filters.filters.is_narrowed() {
            crate::msg::desktop_bills_filter_empty()
        } else {
            crate::msg::desktop_bills_schedule_empty()
        };
        return div()
            .text_color(color::muted(cx))
            .child(empty)
            .into_any_element();
    }
    let last = props.rows.len() - 1;
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
                })),
        )
        .into_any_element()
}

fn table_header(cx: &App) -> impl IntoElement {
    let cell = |width, label: String| div().w(width).flex_none().child(upper(&label));
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
        .child(cell(
            ACCOUNT_WIDTH,
            crate::msg::desktop_bills_column_account(),
        ))
        .child(
            div()
                .w(PLANNED_WIDTH)
                .flex_none()
                .text_align(gpui::TextAlign::Right)
                .child(upper(&crate::msg::desktop_bills_column_planned())),
        )
        .child(
            div()
                .w(ACTUAL_WIDTH)
                .flex_none()
                .text_align(gpui::TextAlign::Right)
                .child(upper(&crate::msg::desktop_bills_column_actual())),
        )
        .child(
            div()
                .w(DUE_WIDTH)
                .flex_none()
                // The handoff runs DUE flush against PLANNED; a gap keeps them apart.
                .pl(px(16.0))
                .child(upper(&crate::msg::desktop_bills_column_due())),
        )
        .child(cell(PAID_WIDTH, crate::msg::desktop_bills_column_paid()))
        .child(cell(
            STATUS_WIDTH,
            crate::msg::desktop_bills_column_status(),
        ))
        .child(
            div()
                .w(ACTIONS_WIDTH)
                .flex_none()
                .text_align(gpui::TextAlign::Right)
                .child(upper(&lib_locale::msg::column_actions())),
        )
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
    row: &ScheduleRow,
    plan: &BillPlan,
    last: bool,
    selected: bool,
    props: &ScheduleProps<'_>,
    cx: &App,
) -> AnyElement {
    let quiet = matches!(row.status, BillStatus::Skipped | BillStatus::Upcoming);
    let colours = RowColours::new(selected, quiet, cx);
    let hover = color::hover(cx);
    let account = props
        .accounts
        .iter()
        .find(|account| account.id == plan.account_id)
        .map_or_else(|| EMPTY_CELL.to_string(), |account| account.name.clone());
    let on_row_click = props.on_row_click.clone();
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
        .when(selected, |this| this.bg(color::selection_background(cx)))
        .when(!selected, |this| this.hover(move |style| style.bg(hover)))
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
                        .when(selected, |this| {
                            this.font_weight(gpui::FontWeight::EXTRA_BOLD)
                        })
                        .child(plan.name.clone()),
                )
                .when(plan.amount_kind == AmountKind::Estimated, |this| {
                    this.child(
                        div()
                            .flex_none()
                            .text_color(colours.secondary)
                            .child(format!("\u{b7} {}", crate::msg::desktop_bills_estimated())),
                    )
                }),
        )
        .child(
            div()
                .w(ACCOUNT_WIDTH)
                .flex_none()
                .truncate()
                .text_color(colours.secondary)
                .child(account),
        )
        .child(
            div()
                .w(PLANNED_WIDTH)
                .flex_none()
                .flex()
                .justify_end()
                .whitespace_nowrap()
                .child(planned_text(row, plan, props)),
        )
        .child(
            div()
                .w(ACTUAL_WIDTH)
                .flex_none()
                .flex()
                .justify_end()
                .whitespace_nowrap()
                .child(actual_text(row, plan, props)),
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
                .child(format_month_day(row.id.due)),
        )
        .child(
            div()
                .w(PAID_WIDTH)
                .flex_none()
                .whitespace_nowrap()
                .text_color(colours.secondary)
                .child(paid_text(row, props)),
        )
        .child(div().w(STATUS_WIDTH).flex_none().flex().child(status_cell(
            row,
            selected,
            &colours,
            props.glyphs,
            cx,
        )))
        .child(
            div()
                .w(ACTIONS_WIDTH)
                .flex_none()
                .flex()
                .justify_end()
                .gap(px(6.0))
                .children(actions_cell(index, row, selected, &colours, props, cx)),
        )
        .into_any_element()
}

/// A row's status: a pill for Overdue (accent) and Due (neutral), `⚑`-prefixed while in Needs
/// Attention; plain text for the rest.
fn status_cell(
    row: &ScheduleRow,
    selected: bool,
    colours: &RowColours,
    glyphs: StatusGlyphs,
    cx: &App,
) -> AnyElement {
    let flag = if row.needs_attention {
        format!("{} ", crate::format::flag_glyph(glyphs))
    } else {
        String::new()
    };
    let pill = |label: String, background: Rgba, text: Rgba| {
        div()
            .px(px(7.0))
            .py(px(2.0))
            .bg(background)
            .text_color(text)
            .text_size(px(10.5))
            .font_weight(gpui::FontWeight::EXTRA_BOLD)
            .child(format!("{flag}{}", upper(&label)))
            .into_any_element()
    };
    let plain = |label: String| {
        div()
            .text_size(px(11.5))
            .text_color(colours.secondary)
            .child(label)
            .into_any_element()
    };
    match row.status {
        BillStatus::Overdue => {
            let (background, text) = if selected {
                (color::accent(cx), color::selection_text(cx))
            } else {
                (color::accent_tint(cx), color::accent_text(cx))
            };
            pill(crate::msg::desktop_bills_status_overdue(), background, text)
        }
        BillStatus::Due => {
            let (background, text) = if selected {
                (color::selection_muted(cx), color::selection_background(cx))
            } else {
                (color::chrome(cx), color::foreground(cx))
            };
            pill(crate::msg::desktop_bills_status_due(), background, text)
        }
        BillStatus::Paid => plain(upper(&crate::msg::desktop_bills_status_paid())),
        BillStatus::Skipped => plain(upper(&crate::msg::desktop_bills_status_skipped())),
        BillStatus::Upcoming => plain(crate::msg::desktop_bills_status_upcoming()),
    }
}

fn actions_cell(
    index: usize,
    row: &ScheduleRow,
    selected: bool,
    colours: &RowColours,
    props: &ScheduleProps<'_>,
    cx: &App,
) -> Vec<AnyElement> {
    let note = |label: String| {
        div()
            .text_size(px(11.0))
            .text_color(colours.secondary)
            .child(label)
            .into_any_element()
    };
    let id = |kind: &str| SharedString::from(format!("bills-{kind}-{index}"));
    let bind = |handler: &OnRowClick| -> OnPlainClick {
        let handler = handler.clone();
        std::rc::Rc::new(move |window: &mut Window, cx: &mut App| handler(index, window, cx))
    };
    match row_actions(row) {
        RowActions::PayOrSkip => {
            // `pay` is the filled one; on the inverted row it flips to the light fill.
            let (pay_background, pay_text) = if selected {
                (color::selection_text(cx), color::selection_background(cx))
            } else {
                (color::foreground(cx), color::background(cx))
            };
            vec![
                action_button(
                    id("pay"),
                    crate::msg::desktop_bills_row_pay(),
                    Some((pay_background, pay_text)),
                    colours.button_border,
                    bind(&props.on_pay_click),
                    cx,
                ),
                action_button(
                    id("skip"),
                    crate::msg::desktop_bills_row_skip(),
                    None,
                    colours.button_border,
                    bind(&props.on_skip_click),
                    cx,
                ),
            ]
        }
        RowActions::ViewTransaction => vec![action_button(
            id("view"),
            crate::msg::desktop_bills_row_view_transaction(),
            None,
            colours.button_border,
            bind(&props.on_view_transaction_click),
            cx,
        )],
        RowActions::Excluded => vec![note(crate::msg::desktop_bills_row_excluded())],
        RowActions::NotActionable => vec![note(crate::msg::desktop_bills_row_not_actionable())],
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

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::bills::EntryId;

    fn row(status: BillStatus, preview: bool) -> ScheduleRow {
        ScheduleRow {
            id: EntryId {
                plan_id: 1,
                due: NaiveDate::from_ymd_opt(2026, 9, 15).unwrap_or_default(),
            },
            status,
            carried: false,
            preview,
            needs_attention: false,
        }
    }

    #[test]
    fn each_status_gets_its_handoff_actions() {
        assert_eq!(
            row_actions(&row(BillStatus::Overdue, false)),
            RowActions::PayOrSkip
        );
        assert_eq!(
            row_actions(&row(BillStatus::Due, false)),
            RowActions::PayOrSkip
        );
        assert_eq!(
            row_actions(&row(BillStatus::Paid, false)),
            RowActions::ViewTransaction
        );
        assert_eq!(
            row_actions(&row(BillStatus::Skipped, false)),
            RowActions::Excluded
        );
        assert_eq!(
            row_actions(&row(BillStatus::Upcoming, false)),
            RowActions::NotActionable
        );
        assert_eq!(
            row_actions(&row(BillStatus::Upcoming, true)),
            RowActions::NotActionable
        );
    }

    #[test]
    fn the_total_names_the_unit_only_off_the_base_unit_and_hides_when_empty() {
        crate::locale::init_for_tests();
        let total = |unit: &str| Total::Single {
            unit: unit.to_string(),
            amount: lib_core::Money(bigdecimal::BigDecimal::new(84250.into(), 2)),
        };
        assert_eq!(
            total_text(&total("aud"), Some("aud")),
            Some("842.50".to_string())
        );
        assert_eq!(
            total_text(&total("usd"), Some("aud")),
            Some("842.50 usd".to_string())
        );
        assert_eq!(total_text(&Total::Empty, Some("aud")), None);
        assert_eq!(
            total_text(&Total::Mixed, Some("aud")),
            Some("mixed units".to_string())
        );
    }
}
