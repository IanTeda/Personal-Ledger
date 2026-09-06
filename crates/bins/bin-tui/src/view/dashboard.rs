//! The Dashboard `View`, hosted by `Shell` (ADR-0013). Wireframe stage: labelled, bordered
//! placeholder boxes matching the pane structure and proportions from
//! `docs/ux/tui/README.md` §2a — net position and its 30-day delta / assets / liabilities
//! trio (item 1, split across two boxes), net worth, income vs expense, where it went,
//! budgets this period, needs attention — so the dashboard's overall layout can be checked
//! and adjusted before any one region's real widget content is built out.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph},
};
use tui_box_text::BoxChar;
use tui_piechart::{PieChart, PieSlice, Resolution, symbols::PIE_CHAR_LIGHT};

use crate::view::{Action, View};

/// The theme's one accent colour — reserved for negatives, over-budget, variance, and
/// Liabilities, per `docs/ux/tui/README.md`'s style table.
const ACCENT: Color = Color::Red;

/// The fake net position figure shown in the headline box, box-text rendered.
const NET_POSITION: &str = "$245,824.10";

/// Column width given to each `BoxChar`: wide enough for its widest glyph (3 cells) plus a
/// spacer column, so consecutive characters don't run into each other.
const BOX_CHAR_WIDTH: u16 = 4;

/// Width of the month-abbreviation column in the income-vs-expense rows (e.g. `"sep "`).
const MONTH_LABEL_WIDTH: u16 = 4;

/// Width of the category-label column in the budget rows — wide enough for the longest fake
/// category (`"subscriptions"`, 13 chars) plus a trailing space; `docs/ux/tui/README.md`
/// suggests 12 cols, sized for shorter real category names.
const BUDGET_LABEL_WIDTH: u16 = 14;

/// Width of the `actual / limit` value column in the budget rows (e.g. `"1,200 / 1,200"`).
const BUDGET_VALUE_WIDTH: u16 = 14;

/// Width of the action column in the needs-attention rows — wide enough for the longest fake
/// action (`":reconcile"`, 10 chars).
const ATTENTION_ACTION_WIDTH: u16 = 10;

/// Rows the needs-attention box always spends on non-item content: the heading, the rule
/// below it, and the trailing "...more" row. Item rows (2-3 per `docs/ux/tui/README.md`)
/// are added on top of this.
const ATTENTION_FIXED_ROWS: u16 = 3;

/// A trivial placeholder Dashboard `View`: labelled boxes proving the §2a pane layout.
#[derive(Default)]
pub struct DashboardView;

impl DashboardView {
    pub fn new() -> Self {
        Self
    }
}

impl View for DashboardView {
    fn update(&mut self, _action: &Action) {}

    fn view(&self, frame: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(4),
                Constraint::Length(1),
                Constraint::Length(18),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(area);

        // rows[0], rows[2], and rows[4] are left blank — breathing space between the
        // shell's title bar and the headline, between the headline and the trend band, and
        // between the trend band and the lower band, not visible rules.
        render_headline(frame, rows[1]);
        render_trend_band(frame, rows[3]);
        render_lower_band(frame, rows[5]);
    }

    fn title(&self) -> &'static str {
        "Dashboard"
    }
}

/// Item 1 — the headline, split into two boxes sharing one row: net position (a "NET
/// POSITION" label over the box-text figure) on the left, and the 30-day delta / assets /
/// liabilities label-over-value trio, bottom-aligned to match, on the right. A terminal has
/// no literal font size, so the box-text figure stands in for the handoff's "double-height
/// or bold" net position treatment from `docs/ux/tui/README.md`.
fn render_headline(frame: &mut Frame, area: Rect) {
    let box_text_width = NET_POSITION.chars().count() as u16 * BOX_CHAR_WIDTH;
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(box_text_width), Constraint::Min(0)])
        .spacing(3)
        .split(area);

    render_net_position(frame, columns[0]);
    render_headline_stats_area(frame, columns[1]);
}

/// The left headline area: "NET POSITION" over the box-text net position figure. No border
/// or title — just the two stacked rows.
fn render_net_position(frame: &mut Frame, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(area);

    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(Paragraph::new(Span::styled("NET POSITION", dim)), rows[0]);
    render_box_text(frame, rows[1], NET_POSITION);
}

/// The right headline area: the 30-day delta / assets / liabilities label-over-value trio,
/// bottom-aligned so its two rows line up with the net position area's last two box-text
/// lines. No border or title.
fn render_headline_stats_area(frame: &mut Frame, area: Rect) {
    let label_row = Rect {
        y: area.y + area.height.saturating_sub(2),
        height: 1,
        ..area
    };
    let value_row = Rect {
        y: area.y + area.height.saturating_sub(1),
        height: 1,
        ..area
    };
    render_headline_stats(frame, label_row, value_row);
}

/// The 30-day delta / assets / liabilities trio, right of the net position figure: a label
/// row and a value row, each split into three columns. Liabilities render in the accent.
fn render_headline_stats(frame: &mut Frame, label_area: Rect, value_area: Rect) {
    let column_widths = [
        Constraint::Length(16),
        Constraint::Length(16),
        Constraint::Length(16),
    ];
    let label_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(column_widths)
        .split(label_area);
    let value_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(column_widths)
        .split(value_area);

    let dim = Style::default().add_modifier(Modifier::DIM);
    for (label, column) in ["30 DAYS", "ASSETS", "LIABILITIES"]
        .into_iter()
        .zip(label_columns.iter())
    {
        frame.render_widget(Paragraph::new(Span::styled(label, dim)), *column);
    }

    let bold = Style::default().add_modifier(Modifier::BOLD);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("+1,268 ", bold),
            Span::styled("(+0.5%)", dim),
        ])),
        value_columns[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled("$75,158", bold)),
        value_columns[1],
    );
    frame.render_widget(
        Paragraph::new(Span::styled("$320,334", bold.fg(ACCENT))),
        value_columns[2],
    );
}

/// Renders `text` using `tui_box_text::BoxChar`, one character per fixed-width column —
/// `BoxChar` only draws a single character per widget call, so composing a string means
/// laying out one `BoxChar` per column slot ourselves. Bolded afterwards: `BoxChar` has no
/// styling API of its own (it only ever sets a cell's symbol, never its style), so the only
/// way to bold it is to patch the modifier onto the buffer once the glyphs are drawn.
fn render_box_text(frame: &mut Frame, area: Rect, text: &str) {
    let constraints: Vec<Constraint> = text
        .chars()
        .map(|_| Constraint::Length(BOX_CHAR_WIDTH))
        .collect();
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(area);

    for (ch, column) in text.chars().zip(columns.iter()) {
        frame.render_widget(&BoxChar::new(ch), *column);
    }

    frame
        .buffer_mut()
        .set_style(area, Style::default().add_modifier(Modifier::BOLD));
}

/// Items 2-3 — the net-worth line chart and the income-vs-expense divergent bars share one
/// 18-row band: the line chart takes roughly 60% of the width, the bars the rest.
fn render_trend_band(frame: &mut Frame, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Min(0)])
        .spacing(3)
        .split(area);

    render_net_worth_chart(frame, columns[0], &fake_net_worth());
    render_income_vs_expense(frame, columns[1], &fake_income_vs_expense());
}

/// One month's fake net-worth balance, feeding the placeholder 18-month line chart.
struct NetWorthPoint {
    /// e.g. `"sep 26"` — only the first, middle, and last months become axis labels.
    month: &'static str,
    balance: f64,
}

/// Item 2 — a placeholder 18-month net-worth line chart: real widget content (a ratatui
/// `Chart`), but still fake data, standing in until the real `lib-database` aggregate is
/// wired up. No border — a "NET WORTH" label sits on its own line above the chart (echoing
/// the "NET POSITION" label pattern in the headline), underlined with a full-width rule
/// rather than boxed. X-axis labelled at the first/middle/last month; no y-axis labels,
/// since a real headline net-position figure will carry the magnitude once one exists.
fn render_net_worth_chart(frame: &mut Frame, area: Rect, net_worth: &[NetWorthPoint]) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);

    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(Paragraph::new(Span::styled("NET WORTH", dim)), rows[0]);
    frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);

    let points: Vec<(f64, f64)> = net_worth
        .iter()
        .enumerate()
        .map(|(index, point)| (index as f64, point.balance))
        .collect();

    let min_balance = points.iter().map(|(_, y)| *y).fold(f64::INFINITY, f64::min);
    let max_balance = points
        .iter()
        .map(|(_, y)| *y)
        .fold(f64::NEG_INFINITY, f64::max);

    let dataset = Dataset::default()
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(ACCENT))
        .data(&points);

    let mid = net_worth.len() / 2;
    let x_labels = [
        net_worth[0].month.to_string(),
        net_worth[mid].month.to_string(),
        net_worth[net_worth.len() - 1].month.to_string(),
    ];

    let chart = Chart::new(vec![dataset])
        .x_axis(
            Axis::default()
                .bounds([0.0, (net_worth.len() - 1) as f64])
                .labels(x_labels),
        )
        .y_axis(Axis::default().bounds([min_balance - 500.0, max_balance + 500.0]));

    frame.render_widget(chart, rows[2]);
}

/// 18 months of a slowly-rising fake balance with a small deterministic wobble — a stand-in
/// for the real net-worth aggregate (FR.34's `Accounts::balance`, summed) a later ticket
/// wires up.
fn fake_net_worth() -> Vec<NetWorthPoint> {
    const MONTHS: [&str; 18] = [
        "apr 25", "may 25", "jun 25", "jul 25", "aug 25", "sep 25", "oct 25", "nov 25", "dec 25",
        "jan 26", "feb 26", "mar 26", "apr 26", "may 26", "jun 26", "jul 26", "aug 26", "sep 26",
    ];

    let mut balance = 58_000.0_f64;
    // A tiny xorshift PRNG: deterministic across runs/platforms, no `rand` dependency needed
    // for fake dashboard data.
    let mut seed: u64 = 11;
    MONTHS
        .iter()
        .map(|&month| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let wobble = (seed % 3000) as f64 - 1500.0;
            balance += 900.0 + wobble * 0.3;
            NetWorthPoint { month, balance }
        })
        .collect()
}

/// One month's fake income/expense pair, feeding the divergent bar chart.
struct MonthFlow {
    month: &'static str,
    income: f64,
    expense: f64,
}

/// Item 3 — 8 months of income vs. expense (more than the handoff's 6, so the month rows
/// exactly fill the Net Worth chart's height, gap-free), one row per month: a two-tone bar
/// split by each month's expense share (accent) vs. income share (default), the same
/// "negatives get the accent" convention as Liabilities. No border — a heading underlined
/// with a full-width rule, matching the Net Worth box, then the month rows (with a blank
/// row between each), then an "expense · income" caption.
fn render_income_vs_expense(frame: &mut Frame, area: Rect, flows: &[MonthFlow]) {
    // `flows.len()` rows plus a 1-row gap between each.
    let month_rows_height = (flows.len() as u16) * 2 - 1;

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(month_rows_height),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(
        Paragraph::new(Span::styled("INCOME VS EXPENSE · 8M DIVERGENT", dim)),
        rows[0],
    );
    frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);

    let month_constraints: Vec<Constraint> = flows.iter().map(|_| Constraint::Length(1)).collect();
    let month_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(month_constraints)
        .spacing(1)
        .split(rows[2]);

    // A shared scale across every month, not a per-row proportion, so bar length reflects
    // each month's actual expense/income rather than just its share of that month's total.
    let max_flow = flows
        .iter()
        .flat_map(|flow| [flow.income, flow.expense])
        .fold(1.0_f64, f64::max);
    for (index, flow) in flows.iter().enumerate() {
        render_divergent_bar_row(frame, month_rows[index], flow, max_flow);
    }

    let caption_row = rows[4];
    let caption_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(MONTH_LABEL_WIDTH), Constraint::Min(0)])
        .split(caption_row);
    let halves = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(caption_columns[1]);
    // A trailing/leading space on each label keeps it a column clear of the halves'
    // shared boundary, so the two words don't run together.
    frame.render_widget(
        Paragraph::new(Span::styled("expense ", dim)).alignment(Alignment::Right),
        halves[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(" income", dim)).alignment(Alignment::Left),
        halves[1],
    );
}

/// One month's row: the month label, then two bars diverging from the centre of the
/// remaining track — expense (accent) growing left, income (default) growing right — both
/// scaled against `max_flow`, a scale shared across every month rather than a per-row
/// proportion, so a bigger number always draws a longer bar.
fn render_divergent_bar_row(frame: &mut Frame, area: Rect, flow: &MonthFlow, max_flow: f64) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(MONTH_LABEL_WIDTH), Constraint::Min(0)])
        .split(area);

    let dim = Style::default().add_modifier(Modifier::DIM);
    frame.render_widget(Paragraph::new(Span::styled(flow.month, dim)), columns[0]);

    // A 1-column gutter at the centre keeps the two bars apart even when both are at
    // (or near) their max length, rather than letting them touch.
    let track = columns[1];
    let usable_width = track.width.saturating_sub(1);
    let left_width = usable_width / 2;
    let right_width = usable_width - left_width;
    let left_half = Rect {
        width: left_width,
        ..track
    };
    let right_half = Rect {
        x: track.x + left_width + 1,
        width: right_width,
        ..track
    };

    let expense_len = ((flow.expense / max_flow) * f64::from(left_width))
        .round()
        .clamp(0.0, f64::from(left_width)) as usize;
    let income_len = ((flow.income / max_flow) * f64::from(right_width))
        .round()
        .clamp(0.0, f64::from(right_width)) as usize;

    // Expense is right-aligned within the left half, so it grows leftward away from the
    // centre as the amount increases, rather than growing rightward toward it.
    let expense_line = Line::from(Span::styled(
        format!(
            "{}{}",
            " ".repeat(left_width as usize - expense_len),
            "█".repeat(expense_len)
        ),
        Style::default().fg(ACCENT),
    ));
    frame.render_widget(Paragraph::new(expense_line), left_half);

    let income_line = Line::from(Span::styled("█".repeat(income_len), Style::default()));
    frame.render_widget(Paragraph::new(income_line), right_half);
}

/// 8 months of fake income/expense totals with varied expense shares — enough rows
/// (`n·2 − 1` months-plus-gaps, plus the heading/rule/caption) to exactly fill the 18-row
/// trend band with no leftover gap, matching the Net Worth chart's height.
fn fake_income_vs_expense() -> Vec<MonthFlow> {
    vec![
        MonthFlow {
            month: "apr",
            income: 7_200.0,
            expense: 1_800.0,
        },
        MonthFlow {
            month: "may",
            income: 4_900.0,
            expense: 4_000.0,
        },
        MonthFlow {
            month: "jun",
            income: 6_000.0,
            expense: 2_600.0,
        },
        MonthFlow {
            month: "jul",
            income: 5_400.0,
            expense: 3_600.0,
        },
        MonthFlow {
            month: "aug",
            income: 4_300.0,
            expense: 5_200.0,
        },
        MonthFlow {
            month: "sep",
            income: 7_900.0,
            expense: 1_400.0,
        },
        MonthFlow {
            month: "oct",
            income: 5_800.0,
            expense: 3_100.0,
        },
        MonthFlow {
            month: "nov",
            income: 6_600.0,
            expense: 4_500.0,
        },
    ]
}

/// Items 4-6 — the pie chart, the budget gauges, and needs-attention share the remaining
/// band: the pie chart takes 2/5 of the screen width; budgets and needs-attention split the
/// rest vertically, needs-attention pinned to the bottom.
fn render_lower_band(frame: &mut Frame, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Min(0)])
        .spacing(3)
        .split(area);

    render_where_it_went(frame, columns[0], &fake_spending());

    let attention_items = fake_attention_items();
    let attention_height = ATTENTION_FIXED_ROWS + attention_items.len() as u16;
    let budgets_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(attention_height)])
        .spacing(1)
        .split(columns[1]);
    render_budgets_this_period(
        frame,
        budgets_rows[0],
        &fake_budgets(),
        &fake_budget_period(),
    );
    render_needs_attention(frame, budgets_rows[1], &attention_items);
}

/// One category's fake 30-day spending share, feeding the pie chart. Largest first,
/// "other" always last, per `docs/ux/tui/README.md`'s legend ordering.
struct SpendingSlice {
    category: &'static str,
    percent: f64,
    color: Color,
}

/// Item 4 — the "Where it went" pie chart: a heading with a "PIE CHART" tag on the right,
/// underlined with a full-width rule (matching Net Worth and Income vs Expense), then the
/// pie itself beside its 5-slice legend (swatch, category, percentage).
fn render_where_it_went(frame: &mut Frame, area: Rect, slices: &[SpendingSlice]) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);

    let dim = Style::default().add_modifier(Modifier::DIM);
    let heading_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(9)])
        .split(rows[0]);
    frame.render_widget(
        Paragraph::new(Span::styled("WHERE IT WENT · 30D", dim)),
        heading_columns[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled("PIE CHART", dim)).alignment(Alignment::Right),
        heading_columns[1],
    );
    frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);
    // rows[2] is left blank — breathing space between the rule and the pie/legend body.

    let body_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Min(0)])
        .spacing(3)
        .split(rows[3]);

    render_pie_chart(frame, body_columns[0], slices);
    render_spending_legend(frame, body_columns[1], slices);
}

/// Draws the pie itself via the `tui-piechart` crate (see GitHub issue #88's charting-library
/// survey), shaded with `PIE_CHAR_LIGHT` (from the crate's `symbols_shades_bars` example) —
/// this needs `Resolution::Standard`, since the crate's `Braille` mode builds its own dot-
/// pattern glyphs and ignores `pie_char` entirely. The crate's own legend is switched off —
/// `render_spending_legend` already matches `docs/ux/tui/README.md`'s exact
/// `swatch category NN%` format (whole-number percentages), which the crate's built-in
/// legend doesn't (it renders one decimal place).
fn render_pie_chart(frame: &mut Frame, area: Rect, slices: &[SpendingSlice]) {
    let pie_slices: Vec<PieSlice> = slices
        .iter()
        .map(|slice| PieSlice::new(slice.category, slice.percent, slice.color))
        .collect();

    let piechart = PieChart::new(pie_slices)
        .resolution(Resolution::Standard)
        .pie_char(PIE_CHAR_LIGHT)
        .show_legend(false);

    frame.render_widget(piechart, area);
}

/// The pie chart's legend: one row per slice, a coloured swatch, the category, and its
/// percentage — largest first, "other" always last.
fn render_spending_legend(frame: &mut Frame, area: Rect, slices: &[SpendingSlice]) {
    let constraints: Vec<Constraint> = slices.iter().map(|_| Constraint::Length(1)).collect();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    for (slice, row) in slices.iter().zip(rows.iter()) {
        let line = Line::from(vec![
            Span::styled("■ ", Style::default().fg(slice.color)),
            Span::raw(format!("{:<10}", slice.category)),
            Span::raw(format!("{:.0}%", slice.percent)),
        ]);
        frame.render_widget(Paragraph::new(line), *row);
    }
}

/// The fake 30-day spending breakdown behind the pie chart and its legend.
fn fake_spending() -> Vec<SpendingSlice> {
    vec![
        SpendingSlice {
            category: "housing",
            percent: 31.0,
            color: ACCENT,
        },
        SpendingSlice {
            category: "groceries",
            percent: 18.0,
            color: Color::Rgb(139, 0, 0),
        },
        SpendingSlice {
            category: "transport",
            percent: 17.0,
            color: Color::DarkGray,
        },
        SpendingSlice {
            category: "utilities",
            percent: 16.0,
            color: Color::Gray,
        },
        SpendingSlice {
            category: "other",
            percent: 18.0,
            color: Color::Rgb(211, 211, 211),
        },
    ]
}

/// One category's fake current-period budget progress, feeding a budget row. Over budget
/// when `actual` exceeds `limit`.
struct BudgetCategory {
    category: &'static str,
    actual: f64,
    limit: f64,
}

/// The current budget period's label and how far through it the period is — shared across
/// every budget row so their `period_progress` ticks all land on the same column.
struct BudgetPeriod {
    /// e.g. `"SEP · DAY 21/30"`.
    label: &'static str,
    elapsed_percent: f64,
}

/// Item 5 — "Budgets this period": up to 5 category rows (label, ratio bar, `actual / limit`
/// figures), largest-first with "other" last, matching the pie chart's legend ordering. Each
/// bar fills by `actual / limit`, clamping full and flipping to the accent when over budget
/// (`docs/ux/tui/README.md`'s over-budget rule). A `│` marks how far the period has
/// elapsed at the same column across every bar, independent of that row's own fill.
fn render_budgets_this_period(
    frame: &mut Frame,
    area: Rect,
    budgets: &[BudgetCategory],
    period: &BudgetPeriod,
) {
    // `budgets.len()` rows plus a 1-row gap between each.
    let category_rows_height = (budgets.len() as u16) * 2 - 1;
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(category_rows_height),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);

    let dim = Style::default().add_modifier(Modifier::DIM);
    let period_tag = format!("{} · {:.0}% ELAPSED", period.label, period.elapsed_percent);
    let heading_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(period_tag.chars().count() as u16),
        ])
        .split(rows[0]);
    frame.render_widget(
        Paragraph::new(Span::styled("BUDGETS THIS PERIOD", dim)),
        heading_columns[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(period_tag, dim)).alignment(Alignment::Right),
        heading_columns[1],
    );
    frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);

    let category_row_constraints: Vec<Constraint> =
        budgets.iter().map(|_| Constraint::Length(1)).collect();
    let category_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(category_row_constraints)
        .spacing(1)
        .split(rows[2]);
    let period_progress = period.elapsed_percent / 100.0;
    for (budget, row) in budgets.iter().zip(category_rows.iter()) {
        render_budget_row(frame, *row, budget, period_progress);
    }

    frame.render_widget(
        Paragraph::new(Span::styled(
            "│ = period progress · bar = actual / limit",
            dim,
        )),
        rows[3],
    );
    // rows[4] is left blank — any leftover height falls below the caption.
}

/// One category's row: its label, a ratio bar, and the `actual / limit` figures right-aligned
/// — both the bar's fill and the figures flip to the accent when over budget.
fn render_budget_row(frame: &mut Frame, area: Rect, budget: &BudgetCategory, period_progress: f64) {
    let is_over = budget.actual > budget.limit;
    let figure_style = if is_over {
        Style::default().fg(ACCENT)
    } else {
        Style::default()
    };

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(BUDGET_LABEL_WIDTH),
            Constraint::Min(0),
            Constraint::Length(BUDGET_VALUE_WIDTH),
        ])
        .spacing(1)
        .split(area);

    frame.render_widget(
        Paragraph::new(Span::styled(budget.category, figure_style)),
        columns[0],
    );
    render_budget_bar(frame, columns[1], budget, period_progress, is_over);
    frame.render_widget(
        Paragraph::new(Span::styled(
            format!(
                "{} / {}",
                format_thousands(budget.actual),
                format_thousands(budget.limit)
            ),
            figure_style,
        ))
        .alignment(Alignment::Right),
        columns[2],
    );
}

/// The ratio bar: filled left-to-right by `actual / limit`, clamped at the bar's full width
/// once over budget, in the accent when over budget and a neutral grey otherwise, with the
/// remainder in a pale shade. A `│` overlays the bar at `period_progress`'s position
/// regardless of the row's own fill, so every row's tick lines up on the same column.
fn render_budget_bar(
    frame: &mut Frame,
    area: Rect,
    budget: &BudgetCategory,
    period_progress: f64,
    is_over: bool,
) {
    let bar_width = area.width as usize;
    if bar_width == 0 {
        return;
    }

    let ratio = (budget.actual / budget.limit).min(1.0);
    let fill_len = (ratio * bar_width as f64).round() as usize;
    let tick_pos = ((period_progress * bar_width as f64).round() as usize).min(bar_width - 1);

    let fill_color = if is_over { ACCENT } else { Color::Gray };
    let spans: Vec<Span> = (0..bar_width)
        .map(|column| {
            if column == tick_pos {
                Span::raw("│")
            } else if column < fill_len {
                Span::styled("█", Style::default().fg(fill_color))
            } else {
                Span::styled("░", Style::default().add_modifier(Modifier::DIM))
            }
        })
        .collect();

    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Formats a whole-dollar amount with thousands separators, e.g. `1200.0` -> `"1,200"`.
fn format_thousands(amount: f64) -> String {
    let digits = (amount.round() as i64).to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, ch) in digits.chars().rev().enumerate() {
        if index != 0 && index % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    grouped.chars().rev().collect()
}

/// The fake current-period budget progress behind the budget rows — largest-limit first,
/// "other" last, echoing the pie chart legend's ordering. "dining" and "subscriptions" are
/// deliberately over budget so the accent/over-budget styling has more than one row to
/// render on.
fn fake_budgets() -> Vec<BudgetCategory> {
    vec![
        BudgetCategory {
            category: "groceries",
            actual: 918.0,
            limit: 1_200.0,
        },
        BudgetCategory {
            category: "dining",
            actual: 412.0,
            limit: 300.0,
        },
        BudgetCategory {
            category: "transport",
            actual: 604.0,
            limit: 1_000.0,
        },
        BudgetCategory {
            category: "utilities",
            actual: 512.0,
            limit: 750.0,
        },
        BudgetCategory {
            category: "housing",
            actual: 1_450.0,
            limit: 1_800.0,
        },
        BudgetCategory {
            category: "subscriptions",
            actual: 86.0,
            limit: 75.0,
        },
        BudgetCategory {
            category: "other",
            actual: 240.0,
            limit: 500.0,
        },
    ]
}

/// The fake current budget period — 70% of the way through September.
fn fake_budget_period() -> BudgetPeriod {
    BudgetPeriod {
        label: "SEP · DAY 21/30",
        elapsed_percent: 70.0,
    }
}

/// One thing needing attention: a description, and the command (or context) that resolves
/// it. `is_alert` flips the description to the accent, for items that are actively wrong
/// (e.g. a balance-check variance) rather than merely outstanding (e.g. unreconciled
/// transactions).
struct AttentionItem {
    description: &'static str,
    action: &'static str,
    is_alert: bool,
}

/// Item 6 — "Needs attention": 2-3 lines only, each naming the command (or context) that
/// resolves it, right-aligned — deliberately not a table (`docs/ux/tui/README.md`). A
/// trailing "...more" row hints at a fuller to-do list beyond what fits here.
fn render_needs_attention(frame: &mut Frame, area: Rect, items: &[AttentionItem]) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(items.len() as u16),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);

    let dim = Style::default().add_modifier(Modifier::DIM);
    let heading_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(7)])
        .split(rows[0]);
    frame.render_widget(
        Paragraph::new(Span::styled("NEEDS ATTENTION", dim)),
        heading_columns[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled("COMMAND", dim)).alignment(Alignment::Right),
        heading_columns[1],
    );
    frame.render_widget(Block::new().borders(Borders::BOTTOM), rows[1]);

    let item_row_constraints: Vec<Constraint> =
        items.iter().map(|_| Constraint::Length(1)).collect();
    let item_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(item_row_constraints)
        .split(rows[2]);
    for (item, row) in items.iter().zip(item_rows.iter()) {
        let description_style = if item.is_alert {
            Style::default().fg(ACCENT)
        } else {
            Style::default()
        };
        render_attention_row(
            frame,
            *row,
            item.description,
            description_style,
            item.action,
        );
    }

    render_attention_row(frame, rows[3], "...more", dim, ":to-do");
    // rows[4] is left blank — any leftover height falls below the "more" row.
}

/// One row: a left-aligned description in `description_style`, and a right-aligned action
/// naming the command (or context) that resolves it, muted — the description is the row's
/// content, the action a quiet hint of how to resolve it.
fn render_attention_row(
    frame: &mut Frame,
    area: Rect,
    description: &str,
    description_style: Style,
    action: &str,
) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(ATTENTION_ACTION_WIDTH),
        ])
        .spacing(1)
        .split(area);

    let action_style = Style::default().add_modifier(Modifier::DIM);

    frame.render_widget(
        Paragraph::new(Span::styled(description, description_style)),
        columns[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(action, action_style)).alignment(Alignment::Right),
        columns[1],
    );
}

/// The fake needs-attention items — an unreconciled-transactions count paired with the
/// command to clear it, and an over-variance balance check paired with the date it was
/// flagged, per `docs/ux/tui/README.md`'s own examples.
fn fake_attention_items() -> Vec<AttentionItem> {
    vec![
        AttentionItem {
            description: "14 unreconciled transactions",
            action: ":reconcile",
            is_alert: false,
        },
        AttentionItem {
            description: "balance check variance -12.40",
            action: "31 aug",
            is_alert: true,
        },
    ]
}

/// A bordered, titled box standing in for a region's real widget content.
fn placeholder(frame: &mut Frame, area: Rect, title: &'static str) {
    frame.render_widget(
        Paragraph::new("placeholder").block(Block::bordered().title(title)),
        area,
    );
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    fn render(view: &DashboardView) -> String {
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| {
                let area = frame.area();
                view.view(frame, area);
            })
            .expect("drawing the dashboard should not error");

        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                text.push_str(buffer[(x, y)].symbol());
            }
            text.push('\n');
        }
        text
    }

    #[test]
    fn renders_without_panicking() {
        render(&DashboardView::new());
    }

    #[test]
    fn shows_all_seven_headline_regions() {
        // The 96x30 minimum from `docs/ux/tui/README.md` doesn't leave enough height for
        // both the "Budgets this period" and "Needs attention" boxes once the latter grows
        // to 3 content lines — the same pre-existing space crunch as
        // `where_it_went_legend_shows_all_five_slices`, so this uses the same taller
        // backend.
        let backend = TestBackend::new(96, 50);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| {
                let area = frame.area();
                DashboardView::new().view(frame, area);
            })
            .expect("drawing the dashboard should not error");
        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                text.push_str(buffer[(x, y)].symbol());
            }
            text.push('\n');
        }

        assert!(text.contains("NET POSITION"), "net position box missing");
        assert!(
            text.contains("30 DAYS"),
            "30-day delta / assets / liabilities box missing"
        );
        assert!(text.contains("NET WORTH"), "net worth box missing");
        assert!(
            text.contains("INCOME VS EXPENSE"),
            "income vs expense box missing"
        );
        assert!(text.contains("WHERE IT WENT"), "pie chart box missing");
        assert!(text.contains("BUDGETS THIS PERIOD"), "budgets box missing");
        assert!(
            text.contains("NEEDS ATTENTION"),
            "needs attention box missing"
        );
    }

    #[test]
    fn net_worth_box_shows_a_placeholder_line_chart() {
        let text = render(&DashboardView::new());

        assert!(text.contains("NET WORTH"), "net worth label missing");
        assert!(text.contains("apr 25"), "first month x-axis label missing");
        assert!(text.contains("sep 26"), "last month x-axis label missing");
        // Braille marker cells the `Chart` line draws with — confirms an actual chart
        // rendered rather than a bare bordered block.
        assert!(
            text.contains(['⠉', '⠊', '⠔', '⠒', '⣀', '⡠']),
            "net worth line chart missing"
        );

        // The box has no border of its own any more — just the "NET WORTH" label with a
        // full-width rule directly beneath it. Row 6 is the label (row 0 is the blank
        // spacer above the headline, rows 1-4 the headline, row 5 the blank spacer below
        // it); row 7 is the rule.
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| {
                let area = frame.area();
                DashboardView::new().view(frame, area);
            })
            .expect("drawing the dashboard should not error");
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(0, 6)].symbol(), "N", "NET WORTH label row missing");
        assert_eq!(buffer[(0, 7)].symbol(), "─", "underline rule missing");
    }

    #[test]
    fn income_vs_expense_shows_eight_months_and_a_caption() {
        let text = render(&DashboardView::new());

        assert!(
            text.contains("INCOME VS EXPENSE · 8M DIVERGENT"),
            "heading missing"
        );
        for month in ["apr", "may", "jun", "jul", "aug", "sep", "oct", "nov"] {
            assert!(text.contains(month), "{month} row missing");
        }
        assert!(text.contains("expense"), "expense caption missing");
        assert!(text.contains("income"), "income caption missing");
    }

    #[test]
    fn income_vs_expense_bars_vary_by_month() {
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| {
                let area = frame.area();
                DashboardView::new().view(frame, area);
            })
            .expect("drawing the dashboard should not error");

        // Each month row splits one bar into an accent-coloured expense span and a default
        // income span; the split point should differ month to month since the fake data
        // gives each month a different expense/income ratio, not a fixed-width bar.
        let buffer = terminal.backend().buffer();
        let expense_len = |y: u16| -> usize {
            (0..buffer.area.width)
                .filter(|&x| buffer[(x, y)].fg == Color::Red)
                .count()
        };

        // Scan every row rather than hardcode which ones are month rows, and assert the
        // resulting expense-span lengths aren't all identical.
        let lengths: Vec<usize> = (0..buffer.area.height).map(expense_len).collect();
        let non_zero: Vec<usize> = lengths.into_iter().filter(|&len| len > 0).collect();
        assert!(
            non_zero.len() >= 6,
            "expected at least 6 rows with an expense span, found {}",
            non_zero.len()
        );
        assert!(
            non_zero
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
                > 1,
            "expense bar lengths should vary by month, all were {non_zero:?}"
        );
    }

    #[test]
    fn headline_shows_the_net_position_as_box_text() {
        let text = render(&DashboardView::new());

        // The `$` glyph's top row, per `tui_box_text`'s character table — confirms the
        // box-text figure actually rendered rather than falling back to plain digits.
        assert!(text.contains("╭┼╴"), "net position box-text figure missing");
        assert!(text.contains("NET POSITION"), "net position label missing");
    }

    #[test]
    fn headline_shows_delta_assets_and_liabilities_right_of_net_position() {
        let text = render(&DashboardView::new());

        assert!(text.contains("30 DAYS"), "30-day delta label missing");
        assert!(text.contains("+1,268"), "30-day delta value missing");
        assert!(text.contains("(+0.5%)"), "30-day delta percentage missing");
        assert!(text.contains("ASSETS"), "assets label missing");
        assert!(text.contains("$75,158"), "assets value missing");
        assert!(text.contains("LIABILITIES"), "liabilities label missing");
        assert!(text.contains("$320,334"), "liabilities value missing");
    }

    #[test]
    fn net_position_box_text_is_bold() {
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| {
                let area = frame.area();
                DashboardView::new().view(frame, area);
            })
            .expect("drawing the dashboard should not error");

        // `BoxChar` has no styling API of its own, so bolding it means patching the buffer
        // after render — this confirms that patch actually lands on a drawn glyph cell. Row
        // 2: row 0 is the title-bar divider, row 1 is the "NET POSITION" label row. Column
        // 1: the `$` glyph's top row is "╭┼╴", flush against the view's left edge.
        let buffer = terminal.backend().buffer();
        let glyph_cell = &buffer[(1, 2)];
        assert_eq!(glyph_cell.symbol(), "┼");
        assert!(glyph_cell.modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn where_it_went_shows_heading_and_pie_chart() {
        let text = render(&DashboardView::new());

        assert!(text.contains("WHERE IT WENT · 30D"), "heading missing");
        assert!(text.contains("PIE CHART"), "pie chart tag missing");
        // `PIE_CHAR_LIGHT` cells the pie draws with — confirms the chart actually rendered,
        // not just a bare heading.
        assert!(text.contains('░'), "pie chart cells missing");
    }

    #[test]
    fn where_it_went_legend_shows_all_five_slices() {
        // The 96x30 minimum from `docs/ux/tui/README.md` doesn't leave enough height for
        // the full 5-row legend once the headline and trend band grow to their current
        // sizes, so this uses a taller backend to check the legend content itself is
        // correct, independent of that pre-existing space crunch.
        let backend = TestBackend::new(96, 50);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| {
                let area = frame.area();
                DashboardView::new().view(frame, area);
            })
            .expect("drawing the dashboard should not error");
        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                text.push_str(buffer[(x, y)].symbol());
            }
            text.push('\n');
        }

        for (category, percent) in [
            ("housing", "31%"),
            ("groceries", "18%"),
            ("transport", "17%"),
            ("utilities", "16%"),
            ("other", "18%"),
        ] {
            assert!(text.contains(category), "{category} row missing");
            assert!(text.contains(percent), "{category}'s percentage missing");
        }
    }

    #[test]
    fn budgets_this_period_shows_heading_and_all_seven_categories() {
        // Rendered in isolation (calling the private render fn directly) rather than through
        // the whole `DashboardView` — "groceries" and "other" both appear a second time in
        // the pie chart's own legend at overlapping row indices, which would make a
        // whole-screen text search ambiguous about which column it matched.
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        let budgets = fake_budgets();
        let period = fake_budget_period();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_budgets_this_period(frame, area, &budgets, &period);
            })
            .expect("drawing budgets this period should not error");

        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                text.push_str(buffer[(x, y)].symbol());
            }
            text.push('\n');
        }

        assert!(text.contains("BUDGETS THIS PERIOD"), "heading missing");
        assert!(text.contains("SEP · DAY 21/30"), "period label missing");
        assert!(text.contains("70% ELAPSED"), "elapsed percentage missing");
        assert!(
            text.contains("│ = period progress · bar = actual / limit"),
            "caption missing"
        );

        for (category, figures) in [
            ("groceries", "918 / 1,200"),
            ("dining", "412 / 300"),
            ("transport", "604 / 1,000"),
            ("utilities", "512 / 750"),
            ("housing", "1,450 / 1,800"),
            ("subscriptions", "86 / 75"),
            ("other", "240 / 500"),
        ] {
            assert!(text.contains(category), "{category} row missing");
            assert!(text.contains(figures), "{category}'s figures missing");
        }
    }

    #[test]
    fn budgets_this_period_flips_over_budget_category_to_the_accent() {
        let backend = TestBackend::new(80, 20);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        let budgets = fake_budgets();
        let period = fake_budget_period();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_budgets_this_period(frame, area, &budgets, &period);
            })
            .expect("drawing budgets this period should not error");

        let buffer = terminal.backend().buffer();
        let row_containing = |needle: &str| -> u16 {
            (0..buffer.area.height)
                .find(|&y| {
                    let mut row = String::new();
                    for x in 0..buffer.area.width {
                        row.push_str(buffer[(x, y)].symbol());
                    }
                    row.contains(needle)
                })
                .unwrap_or_else(|| panic!("no row contains {needle:?}"))
        };
        let row_has_accent =
            |y: u16| -> bool { (0..buffer.area.width).any(|x| buffer[(x, y)].fg == ACCENT) };

        // "dining" and "subscriptions" are over budget and should flip to the accent;
        // "groceries" is under budget and should not.
        assert!(
            row_has_accent(row_containing("dining")),
            "over-budget row should use the accent"
        );
        assert!(
            row_has_accent(row_containing("subscriptions")),
            "over-budget row should use the accent"
        );
        assert!(
            !row_has_accent(row_containing("groceries")),
            "under-budget row should not use the accent"
        );
    }

    #[test]
    fn needs_attention_shows_heading_items_and_the_more_row() {
        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        let items = fake_attention_items();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_needs_attention(frame, area, &items);
            })
            .expect("drawing needs attention should not error");

        let buffer = terminal.backend().buffer();
        let mut text = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                text.push_str(buffer[(x, y)].symbol());
            }
            text.push('\n');
        }

        assert!(text.contains("NEEDS ATTENTION"), "heading missing");
        assert!(text.contains("COMMAND"), "command column heading missing");
        assert!(
            text.contains("14 unreconciled transactions"),
            "unreconciled transactions row missing"
        );
        assert!(text.contains(":reconcile"), "reconcile action missing");
        assert!(
            text.contains("balance check variance -12.40"),
            "balance variance row missing"
        );
        assert!(text.contains("31 aug"), "balance variance date missing");
        assert!(text.contains("...more"), "more row missing");
        assert!(text.contains(":to-do"), "to-do action missing");
    }

    #[test]
    fn needs_attention_flips_alert_item_to_the_accent_and_dims_the_command_column() {
        let backend = TestBackend::new(80, 10);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        let items = fake_attention_items();
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_needs_attention(frame, area, &items);
            })
            .expect("drawing needs attention should not error");

        let buffer = terminal.backend().buffer();
        let row_containing = |needle: &str| -> u16 {
            (0..buffer.area.height)
                .find(|&y| {
                    let mut row = String::new();
                    for x in 0..buffer.area.width {
                        row.push_str(buffer[(x, y)].symbol());
                    }
                    row.contains(needle)
                })
                .unwrap_or_else(|| panic!("no row contains {needle:?}"))
        };
        let row_has_accent =
            |y: u16| -> bool { (0..buffer.area.width).any(|x| buffer[(x, y)].fg == ACCENT) };
        let cell_is_dim =
            |x: u16, y: u16| -> bool { buffer[(x, y)].modifier.contains(Modifier::DIM) };
        let row_ends_dim = |y: u16| -> bool {
            (0..buffer.area.width)
                .rev()
                .find(|&x| buffer[(x, y)].symbol() != " ")
                .is_some_and(|x| cell_is_dim(x, y))
        };

        // "balance check variance" is an alert item and should flip to the accent; the
        // unreconciled-transactions row is not an alert and should not.
        assert!(
            row_has_accent(row_containing("balance check variance")),
            "alert row should use the accent"
        );
        assert!(
            !row_has_accent(row_containing("14 unreconciled")),
            "non-alert row should not use the accent"
        );
        // The whole command column is muted, whether it's a command (":reconcile",
        // ":to-do") or context (the "31 aug" date).
        for needle in [":reconcile", "31 aug", ":to-do"] {
            assert!(
                row_ends_dim(row_containing(needle)),
                "{needle}'s row should end with a dim command column"
            );
        }
    }

    #[test]
    fn title_is_dashboard() {
        assert_eq!(DashboardView::new().title(), "Dashboard");
    }
}
