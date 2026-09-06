//! The Dashboard `View`, hosted by `Shell` (ADR-0013). Builds out the headline row and the
//! trend band — the 18-month net-worth line and the 6-month income-vs-expense divergent bars
//! — from `docs/ux/shell/README.md` §2a, items 1-3. Every figure here is hardcoded fake data;
//! wiring the real `lib-database` aggregates (net position, monthly balances, monthly
//! income/expense) is later work. The doughnut, budget gauges, and "needs attention" rows
//! (items 4-6) are also later work.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Chart, Dataset, GraphType, Paragraph},
};

use crate::view::{Action, View};

/// The theme's one accent colour — reserved for negatives, over-budget, variance, and
/// Liabilities, per `docs/ux/shell/README.md`'s style table. Never used for anything else,
/// so colour never has to carry a distinction on its own.
const ACCENT: Color = Color::Red;

/// One month's fake net-worth balance, feeding the 18-month trend line (item 2).
struct NetWorthPoint {
    /// e.g. `"sep 26"` — only the first, middle, and last months become axis labels.
    month: &'static str,
    balance: f64,
}

/// One month's fake income/expense pair, feeding the 6-month divergent bar chart (item 3).
struct MonthFlow {
    month: &'static str,
    income: f64,
    expense: f64,
}

/// The chart-led dashboard's headline row and trend band.
pub struct DashboardView {
    net_worth: Vec<NetWorthPoint>,
    flows: Vec<MonthFlow>,
}

impl DashboardView {
    pub fn new() -> Self {
        Self {
            net_worth: fake_net_worth(),
            flows: fake_income_vs_expense(),
        }
    }
}

impl Default for DashboardView {
    fn default() -> Self {
        Self::new()
    }
}

impl View for DashboardView {
    fn update(&mut self, _action: &Action) {}

    fn view(&self, frame: &mut Frame, area: Rect) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(0)])
            .split(area);

        render_headline(frame, rows[0]);
        render_trend_band(frame, rows[1], &self.net_worth, &self.flows);
    }

    fn title(&self) -> &'static str {
        "Dashboard"
    }
}

/// Item 1 — four label-over-value columns on one two-row band: net position (bold — the
/// handoff's "double-height or bold"; a terminal has no literal font size, so bold is what
/// keeps it genuinely on the same line as the other three, rather than a multi-row figure
/// towering over them), then 30-day delta, assets, liabilities. Liabilities render in the
/// accent; the delta's percentage carries the handoff's `.mut` (muted) treatment, distinct
/// from the `.lab` labels — both dim, but never confused for one another.
fn render_headline(frame: &mut Frame, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(area);

    let column_widths = [
        Constraint::Length(20),
        Constraint::Length(20),
        Constraint::Length(16),
        Constraint::Length(16),
    ];
    let label_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(column_widths)
        .split(rows[0]);
    let value_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(column_widths)
        .split(rows[1]);

    let dim = Style::default().add_modifier(Modifier::DIM);
    for (label, column) in ["NET POSITION", "30 DAYS", "ASSETS", "LIABILITIES"]
        .into_iter()
        .zip(label_columns.iter())
    {
        frame.render_widget(Paragraph::new(Span::styled(label, dim)), *column);
    }

    let bold = Style::default().add_modifier(Modifier::BOLD);
    frame.render_widget(
        Paragraph::new(Span::styled("$ 245 824.10", bold)),
        value_columns[0],
    );
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("+ 1 268 ", bold),
            Span::styled("(+0.5%)", dim),
        ])),
        value_columns[1],
    );
    frame.render_widget(
        Paragraph::new(Span::styled("75 158", bold)),
        value_columns[2],
    );
    frame.render_widget(
        Paragraph::new(Span::styled("320 334", bold.fg(ACCENT))),
        value_columns[3],
    );
}

/// Items 2-3 — the net-worth line chart and the income-vs-expense divergent bars, sharing one
/// horizontal band: the line chart takes roughly 60% of the width, the bars the rest.
fn render_trend_band(
    frame: &mut Frame,
    area: Rect,
    net_worth: &[NetWorthPoint],
    flows: &[MonthFlow],
) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Min(0)])
        .split(area);

    render_net_worth_chart(frame, columns[0], net_worth);
    render_income_vs_expense(frame, columns[1], flows);
}

/// Item 2 — an 18-month net-worth line, x-axis labelled at the first/middle/last month, no
/// y-axis labels (the headline figure already carries the magnitude).
fn render_net_worth_chart(frame: &mut Frame, area: Rect, net_worth: &[NetWorthPoint]) {
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
        .style(Style::default())
        .data(&points);

    let mid = net_worth.len() / 2;
    let x_labels = [
        net_worth[0].month.to_string(),
        net_worth[mid].month.to_string(),
        net_worth[net_worth.len() - 1].month.to_string(),
    ];

    let chart = Chart::new(vec![dataset])
        .block(Block::bordered().title(" Net worth · 18 months "))
        .x_axis(
            Axis::default()
                .bounds([0.0, (net_worth.len() - 1) as f64])
                .labels(x_labels),
        )
        .y_axis(Axis::default().bounds([min_balance - 500.0, max_balance + 500.0]));

    frame.render_widget(chart, area);
}

/// Item 3 — 6 months of income vs. expense as bars diverging from a centre month gutter:
/// expense grows leftward, income grows rightward, drawn directly as styled text rather than
/// via `Canvas` (README's second suggested approach).
fn render_income_vs_expense(frame: &mut Frame, area: Rect, flows: &[MonthFlow]) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let block = Block::bordered().title(" Income vs expense · 6 months ");
    let inner = block.inner(rows[0]);
    frame.render_widget(block, rows[0]);

    // Each half gets a fixed-width bar track either side of the month gutter. The gutter's
    // own leading/trailing space (see `divergent_bar_row`) is what keeps a full-length bar
    // from running straight into the month text.
    let bar_width = inner
        .width
        .saturating_sub(GUTTER_WIDTH)
        .checked_div(2)
        .unwrap_or(0)
        .clamp(4, 12) as usize;

    let max_flow = flows
        .iter()
        .flat_map(|flow| [flow.income, flow.expense])
        .fold(1.0_f64, f64::max);

    let lines: Vec<Line> = flows
        .iter()
        .map(|flow| divergent_bar_row(flow, max_flow, bar_width))
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);

    let half_columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(rows[1]);
    let dim = Style::default().add_modifier(Modifier::DIM);
    // A trailing/leading space on each label keeps it a column clear of the centre boundary,
    // echoing the gap the bar rows above keep around the month gutter.
    frame.render_widget(
        Paragraph::new(Span::styled("expense ", dim)).alignment(Alignment::Right),
        half_columns[0],
    );
    frame.render_widget(
        Paragraph::new(Span::styled(" income", dim)).alignment(Alignment::Left),
        half_columns[1],
    );
}

/// Width of the month gutter between the two bar tracks: a leading/trailing space either
/// side of a 3-character month abbreviation, so a full-length bar never runs into the text.
const GUTTER_WIDTH: u16 = 5;

/// One row of the divergent bar chart: an expense bar growing left from the gutter, the
/// month abbreviation in the gutter itself, then an income bar growing right.
fn divergent_bar_row(flow: &MonthFlow, max_flow: f64, bar_width: usize) -> Line<'static> {
    let expense_len = bar_length(flow.expense, max_flow, bar_width);
    let income_len = bar_length(flow.income, max_flow, bar_width);

    let expense_bar = format!(
        "{}{}",
        " ".repeat(bar_width - expense_len),
        "█".repeat(expense_len)
    );
    let income_bar = format!(
        "{}{}",
        "█".repeat(income_len),
        " ".repeat(bar_width - income_len)
    );

    Line::from(vec![
        Span::raw(expense_bar),
        Span::raw(format!(" {:^3} ", flow.month)),
        Span::raw(income_bar),
    ])
}

/// Scales a fake dollar amount to a whole number of bar-track cells.
fn bar_length(amount: f64, max_flow: f64, bar_width: usize) -> usize {
    ((amount / max_flow) * bar_width as f64)
        .round()
        .clamp(0.0, bar_width as f64) as usize
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
    // for fake dashboard data (same approach as the FC-TUI-002 line chart demo).
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

/// 6 months of fake income/expense totals, roughly tracking the net-worth trend's tail end.
fn fake_income_vs_expense() -> Vec<MonthFlow> {
    vec![
        MonthFlow {
            month: "apr",
            income: 5_200.0,
            expense: 3_800.0,
        },
        MonthFlow {
            month: "may",
            income: 5_200.0,
            expense: 4_100.0,
        },
        MonthFlow {
            month: "jun",
            income: 5_600.0,
            expense: 3_950.0,
        },
        MonthFlow {
            month: "jul",
            income: 5_200.0,
            expense: 4_700.0,
        },
        MonthFlow {
            month: "aug",
            income: 6_100.0,
            expense: 4_200.0,
        },
        MonthFlow {
            month: "sep",
            income: 5_200.0,
            expense: 3_600.0,
        },
    ]
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
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

    fn render(view: &DashboardView) -> String {
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");
        terminal
            .draw(|frame| {
                let area = frame.area();
                view.view(frame, area);
            })
            .expect("drawing the dashboard should not error");
        buffer_text(&terminal)
    }

    #[test]
    fn renders_without_panicking() {
        render(&DashboardView::new());
    }

    #[test]
    fn headline_shows_net_position_delta_assets_and_liabilities() {
        let text = render(&DashboardView::new());

        assert!(text.contains("NET POSITION"), "net position label missing");
        assert!(text.contains("$ 245 824.10"), "net position value missing");
        assert!(text.contains("30 DAYS"), "30-day delta label missing");
        assert!(text.contains("+ 1 268"), "30-day delta value missing");
        assert!(text.contains("(+0.5%)"), "30-day delta percentage missing");
        assert!(text.contains("ASSETS"), "assets label missing");
        assert!(text.contains("75 158"), "assets value missing");
        assert!(text.contains("LIABILITIES"), "liabilities label missing");
        assert!(text.contains("320 334"), "liabilities value missing");
    }

    #[test]
    fn trend_band_shows_the_net_worth_chart_and_income_expense_bars() {
        let text = render(&DashboardView::new());

        assert!(text.contains("Net worth"), "net-worth chart title missing");
        assert!(text.contains("apr 25"), "first month x-axis label missing");
        assert!(text.contains("sep 26"), "last month x-axis label missing");
        assert!(
            text.contains("Income vs expense"),
            "income/expense chart title missing"
        );
        assert!(text.contains("expense"), "expense direction label missing");
        assert!(text.contains("income"), "income direction label missing");
    }

    #[test]
    fn title_is_dashboard() {
        assert_eq!(DashboardView::new().title(), "Dashboard");
    }
}
