//! The divergent (diverging bar) chart feasibility demo (FC-TUI-002 continued): renders
//! synthetic budget-variance data as bars diverging from a zero line via ratatui's `Canvas`
//! widget. No crate for this chart type exists at all (confirmed in
//! `docs/research/tui-charting-libraries.md`), so per ADR-0002
//! (`docs/adr/0002-ratatui-for-tui-charting.md`) this is fully custom — and deliberately
//! minimal, a plain two-directional bar layout proving the mechanism rather than matching
//! the eventual Concept-cycle's visual polish.

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

/// Half the thickness of each bar, in canvas y-units (one unit per category row).
const BAR_HALF_THICKNESS: f64 = 0.35;
/// Step between stacked horizontal lines used to fill each bar's thickness.
const THICKNESS_STEP: f64 = 0.05;

/// One category's dummy budget-vs-actual variance: positive is under budget, negative is
/// over budget (see FR.37 in `docs/product-requirements.md`).
struct Variance {
    label: &'static str,
    amount: f64,
}

/// Demonstrates a divergent (diverging bar) chart of synthetic budget variance per category.
pub struct DivergentChartScreen {
    variances: Vec<Variance>,
}

impl DivergentChartScreen {
    /// Builds the screen with dummy budget-variance data — a mix of under- and over-budget
    /// categories, so both bar directions are demonstrated.
    pub fn new() -> Self {
        let variances = vec![
            Variance {
                label: "Groceries",
                amount: -45.0,
            },
            Variance {
                label: "Rent",
                amount: 5.0,
            },
            Variance {
                label: "Transport",
                amount: 30.0,
            },
            Variance {
                label: "Entertainment",
                amount: -15.0,
            },
            Variance {
                label: "Utilities",
                amount: 60.0,
            },
        ];
        Self { variances }
    }
}

impl Screen for DivergentChartScreen {
    fn update(&mut self, _action: &Action) {
        // Static dummy data — nothing to react to yet.
    }

    fn title(&self) -> &'static str {
        "Divergent Chart"
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(0), Constraint::Length(26)])
            .split(area);

        let variances = &self.variances;
        let count = variances.len();
        let max_abs = variances
            .iter()
            .map(|variance| variance.amount.abs())
            .fold(0.0_f64, f64::max)
            .max(1.0);
        let x_bound = max_abs * 1.2;

        let canvas = Canvas::default()
            .block(Block::bordered().title(" Divergent chart demo — FC-TUI-002 (dummy data) "))
            .x_bounds([-x_bound, x_bound])
            .y_bounds([0.0, count as f64])
            .paint(move |ctx| {
                // The zero line every bar diverges from.
                ctx.draw(&CanvasLine {
                    x1: 0.0,
                    y1: 0.0,
                    x2: 0.0,
                    y2: count as f64,
                    color: Color::Gray,
                });
                for (index, variance) in variances.iter().enumerate() {
                    // Index 0 at the top, matching the legend's reading order.
                    let row_center = count as f64 - index as f64 - 0.5;
                    let color = if variance.amount >= 0.0 {
                        Color::Green
                    } else {
                        Color::Red
                    };
                    let mut offset = -BAR_HALF_THICKNESS;
                    while offset <= BAR_HALF_THICKNESS {
                        ctx.draw(&CanvasLine {
                            x1: 0.0,
                            y1: row_center + offset,
                            x2: variance.amount,
                            y2: row_center + offset,
                            color,
                        });
                        offset += THICKNESS_STEP;
                    }
                }
            });
        frame.render_widget(canvas, columns[0]);

        let legend_items: Vec<ListItem> = self
            .variances
            .iter()
            .map(|variance| {
                let color = if variance.amount >= 0.0 {
                    Color::Green
                } else {
                    Color::Red
                };
                let sign = if variance.amount >= 0.0 { "+" } else { "" };
                ListItem::new(Span::styled(
                    format!("{:<14}{sign}{:.0}", variance.label, variance.amount),
                    Style::default().fg(color),
                ))
            })
            .collect();
        let legend = List::new(legend_items).block(Block::bordered().title(" Budget variance "));
        frame.render_widget(legend, columns[1]);
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    #[test]
    fn renders_without_panicking() {
        let screen = DivergentChartScreen::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| screen.view(frame, frame.area()))
            .expect("rendering the divergent chart should not error");
    }
}
