//! The Budgets `View`, hosted by `Shell` (ADR-0013). Wireframe stage: a single bordered box
//! proving the navigation path — the command popup's `budget list [period]` entry and the
//! global `g b` jump chord both land here — before any real list/detail content is built out.

use ratatui::{Frame, layout::Rect, widgets::Block};

use crate::view::{Action, View};

/// A trivial placeholder Budgets `View`: one bordered box, no content yet.
#[derive(Default)]
pub struct BudgetsView;

impl BudgetsView {
    pub fn new() -> Self {
        Self
    }
}

impl View for BudgetsView {
    fn update(&mut self, _action: &Action) {}

    fn view(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Block::bordered().title(" Budgets "), area);
    }

    fn title(&self) -> &'static str {
        "Budgets"
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    #[test]
    fn renders_without_panicking() {
        let view = BudgetsView::new();
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| view.view(frame, frame.area()))
            .expect("rendering the placeholder Budgets view should not error");
    }

    #[test]
    fn title_is_budgets() {
        assert_eq!(BudgetsView::new().title(), "Budgets");
    }
}
