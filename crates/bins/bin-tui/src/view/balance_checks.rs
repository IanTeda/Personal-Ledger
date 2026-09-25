//! The Balance Checks `View`, hosted by `Shell` (ADR-0013). Wireframe stage: a single
//! bordered box proving the navigation path — the command popup's `check list` entry and
//! the global `g k` jump chord both land here — before any real list/detail content is built
//! out.

use ratatui::{Frame, layout::Rect, widgets::Block};

use crate::view::{Action, View, ViewId};

/// A trivial placeholder Balance Checks `View`: one bordered box, no content yet.
#[derive(Default)]
pub struct BalanceChecksView;

impl BalanceChecksView {
    pub fn new() -> Self {
        Self
    }
}

impl View for BalanceChecksView {
    fn update(&mut self, _action: &Action) {}

    fn view(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Block::bordered().title(format!(" {} ", self.title())), area);
    }

    fn id(&self) -> ViewId {
        ViewId::BalanceChecks
    }

    fn title(&self) -> String {
        crate::msg::tui_view_balance_checks_title()
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    #[test]
    fn renders_without_panicking() {
        let view = BalanceChecksView::new();
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| view.view(frame, frame.area()))
            .expect("rendering the placeholder Balance Checks view should not error");
    }

    #[test]
    fn title_is_balance_checks() {
        crate::locale::init_for_tests();
        assert_eq!(BalanceChecksView::new().title(), "Balance checks");
        assert_eq!(BalanceChecksView::new().id(), ViewId::BalanceChecks);
    }
}
