//! The Transactions `View`, hosted by `Shell` (ADR-0013). Wireframe stage: a single bordered
//! box proving the navigation path — the command popup's `txn recent` entry and the global
//! `g t` jump chord both land here — before any real list/detail content is built out.

use ratatui::{Frame, layout::Rect, widgets::Block};

use crate::view::{Action, View, ViewId};

/// A trivial placeholder Transactions `View`: one bordered box, no content yet.
#[derive(Default)]
pub struct TransactionsView;

impl TransactionsView {
    pub fn new() -> Self {
        Self
    }
}

impl View for TransactionsView {
    fn update(&mut self, _action: &Action) {}

    fn view(&self, frame: &mut Frame<'_>, area: Rect) {
        frame.render_widget(Block::bordered().title(format!(" {} ", self.title())), area);
    }

    fn id(&self) -> ViewId {
        ViewId::Transactions
    }

    fn title(&self) -> String {
        lib_locale::msg::nav_transactions()
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    #[test]
    fn renders_without_panicking() {
        let view = TransactionsView::new();
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| view.view(frame, frame.area()))
            .expect("rendering the placeholder Transactions view should not error");
    }

    #[test]
    fn title_is_transactions() {
        crate::locale::init_for_tests();
        assert_eq!(TransactionsView::new().title(), "Transactions");
        assert_eq!(TransactionsView::new().id(), ViewId::Transactions);
    }
}
