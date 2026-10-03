//! The Dashboard view interior (`docs/ux/desktop/01-shell/README.md`'s "1a" spec,
//! "View area (Dashboard)" component) -- frame only, per the handoff's own fidelity note:
//! match the headers, column widths, and rules, not the sample data. Every figure below is
//! representative content matching the handoff's own mockup, not real `lib_database` data --
//! wiring a real Ledger's figures in is separate future work (issue #144's "Out of scope"). The
//! exceptions are Needs Attention's Bill rows, which come from the Bills stub through the one
//! shared rule (`bills::attention_entries`, #377), and the budget list, which reads the default
//! Budget through `budgets::period_figures` (#411).

use std::rc::Rc;

use gpui::{App, SharedString, Window, div, prelude::*, px, relative};
use gpui_component::chart::{LineChart, PieChart};

use crate::{bills::EntryId, msg, theme::color};

/// A Bill row's click: hands off to the Bills Schedule tab with that entry selected.
pub type OnBillClick = Rc<dyn Fn(EntryId, &mut Window, &mut App)>;

/// One Bill Schedule entry in Needs Attention, its text already formatted by the Shell.
pub struct AttentionBill {
    pub id: EntryId,
    pub plan: String,
    pub overdue: bool,
    pub due: String,
    pub amount: String,
}

/// One bar of the budget list, its text already formatted by the Shell.
pub struct BudgetBar {
    pub category: String,
    /// "640.00 / 1,000.00".
    pub figures: String,
    /// Spent as a share of the budget, 0 to 1.
    pub fraction: f32,
    pub over: bool,
}

/// The default Budget's list for the current month.
#[derive(Default)]
pub struct BudgetList {
    /// "September 2026 · day 21/30 · 70% elapsed".
    pub period: String,
    /// The month's elapsed share, 0 to 1: the tick on every bar.
    pub elapsed: f32,
    pub bars: Vec<BudgetBar>,
}

#[derive(IntoElement)]
pub struct Dashboard {
    bills: Vec<AttentionBill>,
    flag: &'static str,
    on_bill_click: OnBillClick,
    budgets: BudgetList,
}

impl Dashboard {
    pub fn new(bills: Vec<AttentionBill>, flag: &'static str, on_bill_click: OnBillClick) -> Self {
        Self {
            bills,
            flag,
            on_bill_click,
            budgets: BudgetList::default(),
        }
    }

    /// Sets the budget list from the default Budget's period figures.
    pub fn budgets(mut self, budgets: BudgetList) -> Self {
        self.budgets = budgets;
        self
    }
}

impl RenderOnce for Dashboard {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .pt(px(20.0))
            .px(px(24.0))
            .child(header(cx))
            .child(figure_row(cx))
            .child(div().h(px(2.0)).mt(px(16.0)).bg(color::structural_rule(cx)))
            .child(chart_band(cx))
            .child(div().h(px(1.0)).mt(px(16.0)).bg(color::hairline(cx)))
            .child(lower_band(&self.budgets, cx))
            .child(div().h(px(1.0)).mt(px(14.0)).bg(color::hairline(cx)))
            .child(needs_attention(self, cx))
    }
}

fn kicker(label: String, cx: &App) -> impl IntoElement {
    div()
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::faint_text(cx))
        .mb(px(5.0))
        .child(label)
}

fn header(cx: &App) -> impl IntoElement {
    div()
        .flex()
        .items_baseline()
        .justify_between()
        .child(
            div()
                .text_size(px(19.0))
                .child(msg::desktop_dashboard_title()),
        )
        .child(
            div()
                .text_size(px(11.5))
                .text_color(color::faint_text(cx))
                .child("13 september 2026 · all accounts · aud"),
        )
}

fn figure_row(cx: &App) -> impl IntoElement {
    let secondary = |label: String, value: &'static str, negative: bool| {
        div().child(kicker(label, cx)).child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(15.0))
                .when(negative, |this| this.text_color(color::negative_text(cx)))
                .child(value),
        )
    };

    div()
        .flex()
        .items_end()
        .gap(px(36.0))
        .mt(px(14.0))
        .child(
            div()
                .child(kicker(msg::desktop_dashboard_net_position(), cx))
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(38.0))
                        .child("428,610.22"),
                ),
        )
        .child(
            div()
                .flex()
                .gap(px(28.0))
                .pb(px(4.0))
                .child(secondary(
                    msg::desktop_dashboard_metric_30_day(),
                    "+3,412.08",
                    false,
                ))
                .child(secondary(
                    msg::desktop_dashboard_metric_assets(),
                    "812,240.00",
                    false,
                ))
                .child(secondary(
                    msg::desktop_dashboard_metric_liabilities(),
                    "\u{2212}383,629.78",
                    true,
                )),
        )
}

/// 18 monthly net-worth points, trending up -- enough for `LineChart` to draw a real line,
/// not the mockup's own pre-computed SVG polyline coordinates (which aren't chart input data).
fn net_worth_series() -> Vec<(SharedString, f64)> {
    const MONTHS: [(i32, u32); 18] = [
        (2025, 3),
        (2025, 4),
        (2025, 5),
        (2025, 6),
        (2025, 7),
        (2025, 8),
        (2025, 9),
        (2025, 10),
        (2025, 11),
        (2025, 12),
        (2026, 1),
        (2026, 2),
        (2026, 3),
        (2026, 4),
        (2026, 5),
        (2026, 6),
        (2026, 7),
        (2026, 9),
    ];
    const VALUES: [f64; 18] = [
        341_200.0, 348_600.0, 344_900.0, 361_500.0, 368_100.0, 365_300.0, 376_800.0, 388_400.0,
        382_600.0, 397_200.0, 405_100.0, 400_900.0, 411_600.0, 418_200.0, 415_400.0, 422_800.0,
        426_100.0, 428_610.22,
    ];

    MONTHS
        .into_iter()
        .zip(VALUES)
        .map(|((year, month), value)| {
            let label = lib_locale::format::format_year_month(year, month).to_lowercase();
            (SharedString::from(label), value)
        })
        .collect()
}

struct MonthFlow {
    month: u32,
    expense_pct: f32,
    income_pct: f32,
    /// `true` for the one month the handoff's own mockup flags with the accent color --
    /// representative of an over-budget/anomalous month.
    flagged: bool,
}

const MONTH_FLOWS: &[MonthFlow] = &[
    MonthFlow {
        month: 4,
        expense_pct: 62.0,
        income_pct: 78.0,
        flagged: false,
    },
    MonthFlow {
        month: 5,
        expense_pct: 71.0,
        income_pct: 74.0,
        flagged: false,
    },
    MonthFlow {
        month: 6,
        expense_pct: 55.0,
        income_pct: 81.0,
        flagged: false,
    },
    MonthFlow {
        month: 7,
        expense_pct: 88.0,
        income_pct: 76.0,
        flagged: true,
    },
    MonthFlow {
        month: 8,
        expense_pct: 64.0,
        income_pct: 79.0,
        flagged: false,
    },
    MonthFlow {
        month: 9,
        expense_pct: 48.0,
        income_pct: 52.0,
        flagged: false,
    },
];

fn chart_band(cx: &App) -> impl IntoElement {
    div()
        .flex()
        .gap(px(24.0))
        .pt(px(14.0))
        .child(net_worth_chart(cx))
        .child(in_vs_out(cx))
}

fn net_worth_chart(cx: &App) -> impl IntoElement {
    let series = net_worth_series();
    let first = series.first().map(|(m, _)| m.clone()).unwrap_or_default();
    let last = series.last().map(|(m, _)| m.clone()).unwrap_or_default();
    let mid = series
        .get(series.len() / 2)
        .map(|(m, _)| m.clone())
        .unwrap_or_default();

    div()
        .flex_1()
        .min_w(px(0.0))
        .flex()
        .flex_col()
        .child(
            div()
                .flex()
                .justify_between()
                .items_baseline()
                .mb(px(8.0))
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(10.0))
                        .child(msg::desktop_dashboard_net_worth_title()),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(color::faint_text(cx))
                        .child(msg::desktop_dashboard_net_worth_close()),
                ),
        )
        .child(
            div().h(px(124.0)).child(
                LineChart::new(series)
                    .x(|(month, _)| month.clone())
                    .y(|(_, value)| *value)
                    .stroke(color::chart_series(0, cx))
                    .dot(),
            ),
        )
        .child(
            div()
                .flex()
                .justify_between()
                .text_size(px(11.0))
                .text_color(color::faint_text(cx))
                .mt(px(4.0))
                .child(first)
                .child(mid)
                .child(last),
        )
}

fn in_vs_out(cx: &App) -> impl IntoElement {
    div()
        .w(px(266.0))
        .flex_none()
        .child(
            div().mb(px(8.0)).child(
                div()
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .text_size(px(10.0))
                    .child(msg::desktop_dashboard_in_vs_out_title()),
            ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(5.0))
                .children(MONTH_FLOWS.iter().map(|flow| flow_row(flow, cx))),
        )
        .child(
            div()
                .flex()
                .justify_between()
                .text_size(px(11.0))
                .text_color(color::faint_text(cx))
                .mt(px(6.0))
                .child(msg::desktop_dashboard_in_vs_out_expense())
                .child(msg::desktop_dashboard_in_vs_out_income()),
        )
}

fn flow_row(flow: &MonthFlow, cx: &App) -> impl IntoElement {
    let expense_color = if flow.flagged {
        color::negative(cx)
    } else {
        color::muted(cx)
    };

    div()
        .h(px(13.0))
        .flex()
        .items_center()
        .child(
            div().flex_1().flex().justify_end().child(
                div()
                    .h(px(11.0))
                    .w(relative(flow.expense_pct / 100.0))
                    .bg(expense_color),
            ),
        )
        .child(
            div()
                .w(px(34.0))
                .text_align(gpui::TextAlign::Center)
                .text_size(px(10.5))
                .text_color(color::muted(cx))
                .child(lib_locale::format::format_month(flow.month).to_lowercase()),
        )
        .child(
            div().flex_1().child(
                div()
                    .h(px(11.0))
                    .w(relative(flow.income_pct / 100.0))
                    .bg(color::foreground(cx)),
            ),
        )
}

struct Segment {
    label: &'static str,
    pct: u32,
}

const SEGMENTS: &[Segment] = &[
    Segment {
        label: "housing",
        pct: 34,
    },
    Segment {
        label: "groceries",
        pct: 22,
    },
    Segment {
        label: "transport",
        pct: 16,
    },
    Segment {
        label: "dining",
        pct: 12,
    },
    Segment {
        label: "other",
        pct: 16,
    },
];

fn lower_band(budgets: &BudgetList, cx: &App) -> impl IntoElement {
    div()
        .flex()
        .gap(px(24.0))
        .pt(px(14.0))
        .child(donut(cx))
        .child(budget_list(budgets, cx))
}

fn donut(cx: &App) -> impl IntoElement {
    // `PieChart`'s colour closure must be `'static`, so resolve each series colour up front.
    let slices: Vec<(&'static Segment, gpui::Rgba)> = SEGMENTS
        .iter()
        .enumerate()
        .map(|(index, segment)| (segment, color::chart_series(index, cx)))
        .collect();

    div()
        .w(px(250.0))
        .flex_none()
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .mb(px(10.0))
                .child(msg::desktop_dashboard_where_it_went_title()),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(16.0))
                .child(
                    div().w(px(88.0)).h(px(88.0)).child(
                        PieChart::new(slices.clone())
                            .value(|(segment, _)| segment.pct as f32)
                            .color(|(_, colour)| *colour)
                            .inner_radius(29.0)
                            .outer_radius(44.0),
                    ),
                )
                .child(
                    div()
                        .flex_1()
                        .flex()
                        .flex_col()
                        .gap(px(4.0))
                        .text_size(px(11.5))
                        .children(
                            slices
                                .iter()
                                .map(|(segment, colour)| legend_row(segment, *colour)),
                        ),
                ),
        )
}

fn legend_row(segment: &Segment, colour: gpui::Rgba) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(7.0))
        .child(div().w(px(9.0)).h(px(9.0)).bg(colour))
        .child(div().flex_1().child(segment.label))
        .child(format!("{}%", segment.pct))
}

fn budget_list(budgets: &BudgetList, cx: &App) -> impl IntoElement {
    div()
        .flex_1()
        .min_w(px(0.0))
        .child(
            div()
                .flex()
                .justify_between()
                .items_baseline()
                .mb(px(10.0))
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(10.0))
                        .child(msg::desktop_dashboard_budgets_title()),
                )
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(color::faint_text(cx))
                        .child(budgets.period.clone()),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(7.0))
                .text_size(px(11.5))
                .children(
                    budgets
                        .bars
                        .iter()
                        .map(|bar| budget_row(bar, budgets.elapsed, cx)),
                )
                .when(budgets.bars.is_empty(), |this| {
                    this.child(
                        div()
                            .text_color(color::muted(cx))
                            .child(msg::desktop_dashboard_budgets_none()),
                    )
                })
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(color::faint_text(cx))
                        .mt(px(2.0))
                        .child(msg::desktop_dashboard_budgets_legend()),
                ),
        )
}

fn budget_row(bar: &BudgetBar, elapsed: f32, cx: &App) -> impl IntoElement {
    let over_budget = bar.over;
    let fraction = bar.fraction.clamp(0.0, 1.0);
    let fill_color = if over_budget {
        color::negative(cx)
    } else {
        color::muted(cx)
    };

    div()
        .flex()
        .items_center()
        .gap(px(12.0))
        .child(div().w(px(78.0)).truncate().child(bar.category.clone()))
        .child(
            div()
                .flex_1()
                .h(px(12.0))
                .bg(color::inset_track(cx))
                .relative()
                .child(
                    div()
                        .absolute()
                        .inset(px(0.0))
                        .w(relative(fraction))
                        .bg(fill_color),
                )
                .child(
                    div()
                        .absolute()
                        .top(px(-2.0))
                        .bottom(px(-2.0))
                        .left(relative(elapsed.clamp(0.0, 1.0)))
                        .w(px(2.0))
                        .bg(color::foreground(cx)),
                ),
        )
        .child(
            div()
                .w(px(118.0))
                .text_right()
                .when(over_budget, |this| {
                    this.text_color(color::negative_text(cx))
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                })
                .child(bar.figures.clone()),
        )
}

fn needs_attention(dashboard: Dashboard, cx: &App) -> impl IntoElement {
    let bold = |text: String| div().font_weight(gpui::FontWeight::EXTRA_BOLD).child(text);

    div()
        .pt(px(12.0))
        .flex()
        .flex_col()
        .gap(px(5.0))
        .text_size(px(12.0))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(10.0))
                .mb(px(3.0))
                .child(msg::desktop_dashboard_needs_attention()),
        )
        .children(
            dashboard
                .bills
                .into_iter()
                .map(|bill| bill_row(bill, dashboard.flag, &dashboard.on_bill_click, cx)),
        )
        .child(
            div()
                .flex()
                .gap(px(4.0))
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_color(color::accent_text(cx))
                        .child("14"),
                )
                .child(msg::desktop_dashboard_unreconciled("ANZ Everyday", 14i64))
                .child(bold(":reconcile".into())),
        )
        .child(
            div()
                .flex()
                .gap(px(4.0))
                .child(bold("3".into()))
                .child(msg::desktop_dashboard_flagged_transactions(3i64))
                .child(bold(":txn recent".into())),
        )
}

/// A Bill in Needs Attention: `⚑ Netflix overdue since 18 Sept, 22.99 →`, the flag in the accent
/// text colour as on the Schedule tab.
fn bill_row(
    bill: AttentionBill,
    flag: &'static str,
    on_click: &OnBillClick,
    cx: &App,
) -> impl IntoElement {
    let text = if bill.overdue {
        msg::desktop_dashboard_bill_overdue(&bill.plan, &bill.due, &bill.amount)
    } else {
        msg::desktop_dashboard_bill_due(&bill.plan, &bill.due, &bill.amount)
    };
    let id = bill.id;
    let on_click = on_click.clone();
    div()
        .debug_selector(move || format!("dashboard-bill-{}-{}", id.plan_id, id.due))
        .id(SharedString::from(format!(
            "dashboard-bill-{}-{}",
            id.plan_id, id.due
        )))
        .flex()
        .gap(px(4.0))
        .cursor_pointer()
        .hover(|this| this.bg(color::hover(cx)))
        .on_click(move |_event, window, cx| on_click(id, window, cx))
        .child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_color(color::accent_text(cx))
                .child(flag),
        )
        .child(text)
}

#[cfg(test)]
mod tests {
    use lib_locale::{Locale, with_locale};

    use super::*;

    fn labels() -> Vec<String> {
        net_worth_series()
            .into_iter()
            .map(|(label, _)| label.to_string())
            .collect()
    }

    #[test]
    fn the_net_worth_axis_names_months_in_the_locale() {
        with_locale(Locale::EnAu, || {
            let labels = labels();
            assert_eq!(labels.len(), 18);
            assert_eq!(labels[0], "mar 2025");
            assert_eq!(labels[17], "sept 2026");
        });
        with_locale(Locale::EnUs, || {
            assert_eq!(labels()[17], "sep 2026");
        });
    }

    #[test]
    fn the_in_vs_out_months_are_real_months_named_in_the_locale() {
        with_locale(Locale::EnAu, || {
            let names: Vec<String> = MONTH_FLOWS
                .iter()
                .map(|flow| lib_locale::format::format_month(flow.month).to_lowercase())
                .collect();
            assert_eq!(names, ["apr", "may", "jun", "jul", "aug", "sept"]);
        });
    }
}
