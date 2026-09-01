//! The line chart feasibility demo (FC-TUI-002): renders synthetic account-balance data over
//! time using ratatui's built-in `Chart` widget, proving line-chart rendering works.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    symbols,
    widgets::{Axis, Block, Chart, Dataset, GraphType},
};

use crate::{action::Action, screen::Screen};

/// Number of synthetic daily balance points to plot.
const POINT_COUNT: usize = 60;

/// A single (day, balance) point plotted on the chart.
type Point = (f64, f64);

/// Demonstrates a line chart of a synthetic account balance trending upward with noise.
pub struct LineChartScreen {
    points: Vec<Point>,
}

impl LineChartScreen {
    /// Builds the screen with deterministic dummy data — a slowly-rising balance with a
    /// small pseudo-random wobble, standing in for a real account's transaction history.
    pub fn new() -> Self {
        let mut balance = 1_000.0_f64;
        // A tiny xorshift PRNG: deterministic across runs/platforms, no `rand` dependency
        // needed for a feasibility demo.
        let mut seed: u64 = 7;
        let points = (0..POINT_COUNT)
            .map(|day| {
                seed ^= seed << 13;
                seed ^= seed >> 7;
                seed ^= seed << 17;
                let wobble = (seed % 200) as f64 - 100.0;
                balance += 15.0 + wobble;
                (day as f64, balance)
            })
            .collect();
        Self { points }
    }
}

impl Screen for LineChartScreen {
    fn update(&mut self, _action: &Action) {
        // Static dummy data — nothing to react to yet.
    }

    fn title(&self) -> &'static str {
        "Line Chart"
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        let min_balance = self
            .points
            .iter()
            .map(|(_, balance)| *balance)
            .fold(f64::INFINITY, f64::min);
        let max_balance = self
            .points
            .iter()
            .map(|(_, balance)| *balance)
            .fold(f64::NEG_INFINITY, f64::max);

        let dataset = Dataset::default()
            .name("Balance")
            .marker(symbols::Marker::Braille)
            .graph_type(GraphType::Line)
            .style(Style::default().fg(Color::Cyan))
            .data(&self.points);

        let chart = Chart::new(vec![dataset])
            .block(Block::bordered().title(" Line chart demo — FC-TUI-002 (dummy data) "))
            .x_axis(
                Axis::default()
                    .title("Day")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([0.0, POINT_COUNT as f64])
                    .labels(["0".to_string(), POINT_COUNT.to_string()]),
            )
            .y_axis(
                Axis::default()
                    .title("Balance ($)")
                    .style(Style::default().fg(Color::Gray))
                    .bounds([min_balance - 50.0, max_balance + 50.0])
                    .labels([format!("{min_balance:.0}"), format!("{max_balance:.0}")]),
            );

        frame.render_widget(chart, area);
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    #[test]
    fn renders_without_panicking() {
        let screen = LineChartScreen::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| screen.view(frame, frame.area()))
            .expect("rendering the line chart should not error");
    }
}
