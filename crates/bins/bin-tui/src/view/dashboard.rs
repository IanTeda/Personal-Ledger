//! The Dashboard `View`, hosted by `Shell` (ADR-0013). Wireframe stage: six labelled,
//! bordered placeholder boxes matching the pane structure and proportions from
//! `docs/ux/shell/README.md` §2a — headline, net worth, income vs expense, where it went,
//! budgets this period, needs attention — so the dashboard's overall layout can be checked
//! and adjusted before any one region's real widget content is built out.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Paragraph},
};

use crate::view::{Action, View};

/// A trivial placeholder Dashboard `View`: six labelled boxes proving the §2a pane layout.
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
                Constraint::Length(3),
                Constraint::Length(8),
                Constraint::Min(0),
                Constraint::Length(3),
            ])
            .split(area);

        placeholder(
            frame,
            rows[0],
            " Headline — net position · 30-day delta · assets · liabilities ",
        );
        render_trend_band(frame, rows[1]);
        render_lower_band(frame, rows[2]);
        placeholder(frame, rows[3], " Needs attention ");
    }

    fn title(&self) -> &'static str {
        "Dashboard"
    }
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
    fn shows_all_six_region_boxes() {
        let text = render(&DashboardView::new());

        assert!(text.contains("Headline"), "headline box missing");
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
    fn title_is_dashboard() {
        assert_eq!(DashboardView::new().title(), "Dashboard");
    }
}
