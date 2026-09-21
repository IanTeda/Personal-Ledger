//! The Dashboard view interior (`docs/ux/desktop/Shell & Navigation/README.md`'s "1a" spec,
//! "View area (Dashboard)" component) -- frame only, per the handoff's own fidelity note:
//! match the headers, column widths, and rules, not the sample data. Every figure below is
//! representative content matching the handoff's own mockup, not real `lib_database` data --
//! wiring a real Ledger's figures in is separate future work (issue #144's "Out of scope").

use gpui::{App, SharedString, Window, div, prelude::*, px, relative};
use gpui_component::chart::{LineChart, PieChart};

use crate::{msg, theme::color};

#[derive(IntoElement)]
pub struct Dashboard;

impl Dashboard {
    pub fn new() -> Self {
        Self
    }
}

impl Default for Dashboard {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for Dashboard {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .pt(px(20.0))
            .px(px(24.0))
            .child(header())
            .child(figure_row())
            .child(div().h(px(2.0)).mt(px(16.0)).bg(color::STRUCTURAL_RULE))
            .child(chart_band())
            .child(div().h(px(1.0)).mt(px(16.0)).bg(color::HAIRLINE))
            .child(lower_band())
            .child(div().h(px(1.0)).mt(px(14.0)).bg(color::HAIRLINE))
            .child(needs_attention())
    }
}

fn kicker(label: &'static str) -> impl IntoElement {
    div()
        .font_weight(gpui::FontWeight::EXTRA_BOLD)
        .text_size(px(10.0))
        .text_color(color::INK_TERTIARY)
        .mb(px(5.0))
        .child(label)
}

fn header() -> impl IntoElement {
    div()
        .flex()
        .items_baseline()
        .justify_between()
        .child(div().text_size(px(19.0)).child(msg::desktop_dashboard_title()))
        .child(
            div()
                .text_size(px(11.5))
                .text_color(color::INK_TERTIARY)
                .child("13 september 2026 · all accounts · aud"),
        )
}

fn figure_row() -> impl IntoElement {
    let secondary = |label, value: &'static str, negative: bool| {
        div().child(kicker(label)).child(
            div()
                .font_weight(gpui::FontWeight::EXTRA_BOLD)
                .text_size(px(15.0))
                .when(negative, |this| this.text_color(color::ACCENT_TEXT))
                .child(value),
        )
    };

    let net_position = msg::desktop_dashboard_net_position();
    let metric_30_day = msg::desktop_dashboard_metric_30_day();
    let metric_assets = msg::desktop_dashboard_metric_assets();
    let metric_liabilities = msg::desktop_dashboard_metric_liabilities();

    div()
        .flex()
        .items_end()
        .gap(px(36.0))
        .mt(px(14.0))
        .child(
            div()
                .child(div()
                    .font_weight(gpui::FontWeight::EXTRA_BOLD)
                    .text_size(px(10.0))
                    .text_color(color::INK_TERTIARY)
                    .mb(px(5.0))
                    .child(net_position))
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
                .child(
                    div().child(div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(10.0))
                        .text_color(color::INK_TERTIARY)
                        .mb(px(5.0))
                        .child(metric_30_day)).child(
                        div()
                            .font_weight(gpui::FontWeight::EXTRA_BOLD)
                            .text_size(px(15.0))
                            .child("+3,412.08"),
                    )
                )
                .child(
                    div().child(div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(10.0))
                        .text_color(color::INK_TERTIARY)
                        .mb(px(5.0))
                        .child(metric_assets)).child(
                        div()
                            .font_weight(gpui::FontWeight::EXTRA_BOLD)
                            .text_size(px(15.0))
                            .child("812,240.00"),
                    )
                )
                .child(
                    div().child(div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_size(px(10.0))
                        .text_color(color::INK_TERTIARY)
                        .mb(px(5.0))
                        .child(metric_liabilities)).child(
                        div()
                            .font_weight(gpui::FontWeight::EXTRA_BOLD)
                            .text_size(px(15.0))
                            .text_color(color::ACCENT_TEXT)
                            .child("\u{2212}383,629.78"),
                    )
                ),
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

fn chart_band() -> impl IntoElement {
    div()
        .flex()
        .gap(px(24.0))
        .pt(px(14.0))
        .child(net_worth_chart())
        .child(in_vs_out())
}

fn net_worth_chart() -> impl IntoElement {
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
                        .text_color(color::INK_TERTIARY)
                        .child(msg::desktop_dashboard_net_worth_close()),
                ),
        )
        .child(
            div().h(px(124.0)).child(
                LineChart::new(series)
                    .x(|(month, _)| month.clone())
                    .y(|(_, value)| *value)
                    .stroke(color::INK)
                    .dot(),
            ),
        )
        .child(
            div()
                .flex()
                .justify_between()
                .text_size(px(11.0))
                .text_color(color::INK_TERTIARY)
                .mt(px(4.0))
                .child(first)
                .child(mid)
                .child(last),
        )
}

fn in_vs_out() -> impl IntoElement {
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
                .children(MONTH_FLOWS.iter().map(flow_row)),
        )
        .child(
            div()
                .flex()
                .justify_between()
                .text_size(px(11.0))
                .text_color(color::INK_TERTIARY)
                .mt(px(6.0))
                .child(msg::desktop_dashboard_in_vs_out_expense())
                .child(msg::desktop_dashboard_in_vs_out_income()),
        )
}

fn flow_row(flow: &MonthFlow) -> impl IntoElement {
    let expense_color = if flow.flagged {
        color::ACCENT
    } else {
        color::INK_SECONDARY
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
                .text_color(color::INK_SECONDARY)
                .child(lib_locale::format::format_month(flow.month).to_lowercase()),
        )
        .child(
            div().flex_1().child(
                div()
                    .h(px(11.0))
                    .w(relative(flow.income_pct / 100.0))
                    .bg(color::INK),
            ),
        )
}

struct Segment {
    label: &'static str,
    pct: u32,
    color: gpui::Rgba,
}

const SEGMENTS: &[Segment] = &[
    Segment {
        label: "housing",
        pct: 34,
        color: color::ACCENT,
    },
    Segment {
        label: "groceries",
        pct: 22,
        color: color::INK,
    },
    Segment {
        label: "transport",
        pct: 16,
        color: color::INK_SECONDARY,
    },
    Segment {
        label: "dining",
        pct: 12,
        color: color::INK_TERTIARY,
    },
    Segment {
        label: "other",
        pct: 16,
        color: color::HAIRLINE,
    },
];

fn lower_band() -> impl IntoElement {
    div()
        .flex()
        .gap(px(24.0))
        .pt(px(14.0))
        .child(donut())
        .child(budget_list())
}

fn donut() -> impl IntoElement {
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
                        PieChart::new(SEGMENTS.iter())
                            .value(|segment| segment.pct as f32)
                            .color(|segment| segment.color)
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
                        .children(SEGMENTS.iter().map(legend_row)),
                ),
        )
}

fn legend_row(segment: &Segment) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(7.0))
        .child(div().w(px(9.0)).h(px(9.0)).bg(segment.color))
        .child(div().flex_1().child(segment.label))
        .child(format!("{}%", segment.pct))
}

struct BudgetRow {
    category: &'static str,
    spent: u32,
    limit: u32,
}

const BUDGET_ROWS: &[BudgetRow] = &[
    BudgetRow {
        category: "groceries",
        spent: 640,
        limit: 1_000,
    },
    BudgetRow {
        category: "dining",
        spent: 412,
        limit: 300,
    },
    BudgetRow {
        category: "transport",
        spent: 208,
        limit: 400,
    },
    BudgetRow {
        category: "utilities",
        spent: 445,
        limit: 550,
    },
];

/// The period's own elapsed fraction ("sep · day 21/30 · 70% elapsed") -- the same marker
/// position on every row, since it marks a point in time, not a per-category value.
const PERIOD_ELAPSED: f32 = 0.70;

fn budget_list() -> impl IntoElement {
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
                        .text_color(color::INK_TERTIARY)
                        .child("sep · day 21/30 · 70% elapsed"),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap(px(7.0))
                .text_size(px(11.5))
                .children(BUDGET_ROWS.iter().map(budget_row))
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(color::INK_TERTIARY)
                        .mt(px(2.0))
                        .child(msg::desktop_dashboard_budgets_legend()),
                ),
        )
}

fn budget_row(row: &BudgetRow) -> impl IntoElement {
    let over_budget = row.spent > row.limit;
    let fraction = (row.spent as f32 / row.limit as f32).min(1.0);
    let fill_color = if over_budget {
        color::ACCENT
    } else {
        color::INK_SECONDARY
    };

    div()
        .flex()
        .items_center()
        .gap(px(12.0))
        .child(div().w(px(78.0)).child(row.category))
        .child(
            div()
                .flex_1()
                .h(px(12.0))
                .bg(color::INSET_TRACK)
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
                        .left(relative(PERIOD_ELAPSED))
                        .w(px(2.0))
                        .bg(color::INK),
                ),
        )
        .child(
            div()
                .w(px(118.0))
                .text_right()
                .when(over_budget, |this| {
                    this.text_color(color::ACCENT_TEXT)
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                })
                .child(format!("{} / {}", row.spent, row.limit)),
        )
}

fn needs_attention() -> impl IntoElement {
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
        .child(
            div()
                .flex()
                .gap(px(4.0))
                .child(
                    div()
                        .font_weight(gpui::FontWeight::EXTRA_BOLD)
                        .text_color(color::ACCENT_TEXT)
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
