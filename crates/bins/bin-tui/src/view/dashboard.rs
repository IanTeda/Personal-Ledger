//! A trivial placeholder Dashboard `View`, hosted by the new `Shell` (ADR-0013). Proves the
//! shell chrome and view region render correctly; the chart-led dashboard specified in
//! `docs/ux/shell/README.md` (net position, net-worth chart, budgets, ...) is later work.

use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Paragraph},
};

use crate::view::{Action, View};

/// Placeholder for the chart-led dashboard the shell design specifies.
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
        let block = Block::bordered().title(" Dashboard ");
        frame.render_widget(
            Paragraph::new("Placeholder — chart-led dashboard coming soon.").block(block),
            area,
        );
    }

    fn title(&self) -> &'static str {
        "Dashboard"
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    #[test]
    fn renders_without_panicking() {
        let view = DashboardView::new();
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| {
                let area = frame.area();
                view.view(frame, area);
            })
            .expect("drawing the placeholder dashboard should not error");
    }

    #[test]
    fn title_is_dashboard() {
        assert_eq!(DashboardView::new().title(), "Dashboard");
    }
}
