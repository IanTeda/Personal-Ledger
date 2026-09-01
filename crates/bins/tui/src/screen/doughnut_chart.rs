//! The doughnut chart feasibility demo (FC-TUI-002 continued): renders a hollow-centre pie
//! chart of synthetic category spending via ratatui's `Canvas` widget with custom radial-line
//! drawing. No off-the-shelf ratatui crate produces an actual doughnut — `tui-piechart` is
//! solid-pie only — so this is deliberately custom, per ADR-0002
//! (`docs/adr/0002-ratatui-for-tui-charting.md`).

use std::f64::consts::PI;

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{
        Block, List, ListItem,
        canvas::{Canvas, Line as CanvasLine},
    },
};

use crate::{action::Action, screen::Screen};

/// The hole in the middle of the doughnut, in canvas units (outer radius is `1.0`).
const INNER_RADIUS: f64 = 0.5;
const OUTER_RADIUS: f64 = 1.0;
/// Angular step between radial lines; fine enough for adjacent lines to look like a filled
/// ring at typical terminal/braille resolution.
const ANGLE_STEP: f64 = 0.015;

/// One category's dummy spending slice.
struct Slice {
    label: &'static str,
    amount: f64,
    color: Color,
}

/// Demonstrates a doughnut (hollow-centre pie) chart of synthetic category spending.
pub struct DoughnutChartScreen {
    slices: Vec<Slice>,
}

impl DoughnutChartScreen {
    /// Builds the screen with dummy category-spending data, in the spirit of Personal
    /// Ledger's own accounting categories (see `CONTEXT.md`).
    pub fn new() -> Self {
        let slices = vec![
            Slice {
                label: "Rent",
                amount: 1_200.0,
                color: Color::Red,
            },
            Slice {
                label: "Groceries",
                amount: 320.0,
                color: Color::Green,
            },
            Slice {
                label: "Utilities",
                amount: 180.0,
                color: Color::Yellow,
            },
            Slice {
                label: "Transport",
                amount: 150.0,
                color: Color::Blue,
            },
            Slice {
                label: "Entertainment",
                amount: 90.0,
                color: Color::Magenta,
            },
        ];
        Self { slices }
    }

    fn total(&self) -> f64 {
        self.slices.iter().map(|slice| slice.amount).sum()
    }
}

impl Screen for DoughnutChartScreen {
    fn update(&mut self, _action: &Action) {
        // Static dummy data — nothing to react to yet.
    }

    fn title(&self) -> &'static str {
        "Doughnut Chart"
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(26)])
            .split(area);

        let total = self.total();
        let slices = &self.slices;

        let canvas = Canvas::default()
            .block(Block::bordered().title(" Doughnut chart demo — FC-TUI-002 (dummy data) "))
            .x_bounds([-1.3, 1.3])
            .y_bounds([-1.3, 1.3])
            .paint(move |ctx| {
                let mut angle = 0.0_f64;
                for slice in slices {
                    let sweep = (slice.amount / total) * 2.0 * PI;
                    let end = angle + sweep;
                    let mut a = angle;
                    while a < end {
                        // Clockwise from the top (12 o'clock), matching conventional pie
                        // chart orientation.
                        let theta = PI / 2.0 - a;
                        let (sin_t, cos_t) = theta.sin_cos();
                        ctx.draw(&CanvasLine {
                            x1: INNER_RADIUS * cos_t,
                            y1: INNER_RADIUS * sin_t,
                            x2: OUTER_RADIUS * cos_t,
                            y2: OUTER_RADIUS * sin_t,
                            color: slice.color,
                        });
                        a += ANGLE_STEP;
                    }
                    angle = end;
                }
            });
        frame.render_widget(canvas, columns[0]);

        let legend_items: Vec<ListItem> = self
            .slices
            .iter()
            .map(|slice| {
                let percentage = slice.amount / total * 100.0;
                ListItem::new(Span::styled(
                    format!(
                        "{:<14}${:>7.0} ({percentage:.0}%)",
                        slice.label, slice.amount
                    ),
                    Style::default().fg(slice.color),
                ))
            })
            .collect();
        let legend = List::new(legend_items).block(Block::bordered().title(" Categories "));
        frame.render_widget(legend, columns[1]);
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    #[test]
    fn renders_without_panicking() {
        let screen = DoughnutChartScreen::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| screen.view(frame, frame.area()))
            .expect("rendering the doughnut chart should not error");
    }
}
