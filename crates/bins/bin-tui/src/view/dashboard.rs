//! The Dashboard `View`, hosted by `Shell` (ADR-0013). Wireframe stage: labelled, bordered
//! placeholder boxes matching the pane structure and proportions from
//! `docs/ux/shell/README.md` §2a — net position and its 30-day delta / assets / liabilities
//! trio (item 1, split across two boxes), net worth, income vs expense, where it went,
//! budgets this period, needs attention — so the dashboard's overall layout can be checked
//! and adjusted before any one region's real widget content is built out.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};
use tui_box_text::BoxChar;

use crate::view::{Action, View};

/// The theme's one accent colour — reserved for negatives, over-budget, variance, and
/// Liabilities, per `docs/ux/shell/README.md`'s style table.
const ACCENT: Color = Color::Red;

/// The fake net position figure shown in the headline box, box-text rendered.
const NET_POSITION: &str = "$245,824.10";

/// Column width given to each `BoxChar`: wide enough for its widest glyph (3 cells) plus a
/// spacer column, so consecutive characters don't run into each other.
const BOX_CHAR_WIDTH: u16 = 4;

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
                Constraint::Length(8),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(area);

        // rows[0] is left blank — a row of breathing space between the shell's title bar
        // (rendered above this view) and the dashboard content, not a visible rule.
        render_headline(frame, rows[1]);
        render_trend_band(frame, rows[2]);
        render_lower_band(frame, rows[3]);
        placeholder(frame, rows[4], " Needs attention ");
    }

    fn title(&self) -> &'static str {
        "Dashboard"
    }
}

/// Item 1 — the headline, split into two boxes sharing one row: net position (a "NET
/// POSITION" label over the box-text figure) on the left, and the 30-day delta / assets /
/// liabilities label-over-value trio, bottom-aligned to match, on the right. A terminal has
/// no literal font size, so the box-text figure stands in for the handoff's "double-height
/// or bold" net position treatment from `docs/ux/shell/README.md`.
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
/// 8-row band: the line chart takes roughly 60% of the width, the bars the rest.
fn render_trend_band(frame: &mut Frame, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Min(0)])
        .split(area);

    placeholder(frame, columns[0], " Net worth · 18 months ");
    placeholder(frame, columns[1], " Income vs expense · 6 months ");
}

/// Items 4-5 — the doughnut and the budget gauges share the remaining band: the doughnut
/// takes a fixed ~26 columns, budgets the rest.
fn render_lower_band(frame: &mut Frame, area: Rect) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(26), Constraint::Min(0)])
        .split(area);

    placeholder(frame, columns[0], " Where it went · 30 days ");
    placeholder(frame, columns[1], " Budgets this period ");
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
        let text = render(&DashboardView::new());

        assert!(text.contains("NET POSITION"), "net position box missing");
        assert!(
            text.contains("30 DAYS"),
            "30-day delta / assets / liabilities box missing"
        );
        assert!(text.contains("Net worth"), "net worth box missing");
        assert!(
            text.contains("Income vs expense"),
            "income vs expense box missing"
        );
        assert!(text.contains("Where it went"), "doughnut box missing");
        assert!(text.contains("Budgets this period"), "budgets box missing");
        assert!(
            text.contains("Needs attention"),
            "needs attention box missing"
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
    fn title_is_dashboard() {
        assert_eq!(DashboardView::new().title(), "Dashboard");
    }
}
