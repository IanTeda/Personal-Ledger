//! The Reports `View`, hosted by `Shell` (ADR-0013). Wireframe stage: a single bordered box
//! proving the navigation path — the command popup's `report list` entry and the global
//! `g r` jump chord both land here — before any real report content is built out.

use ratatui::{Frame, layout::Rect, widgets::Block};

use crate::view::{Action, View, ViewId};

/// A trivial placeholder Reports `View`: one bordered box, no content yet.
#[derive(Default)]
pub struct ReportsView;

impl ReportsView {
    pub fn new() -> Self {
        Self
    }
}

impl View for ReportsView {
    fn update(&mut self, _action: &Action) {}

    fn view(&self, frame: &mut Frame<'_>, area: Rect) {
        frame.render_widget(Block::bordered().title(format!(" {} ", self.title())), area);
    }

    fn id(&self) -> ViewId {
        ViewId::Reports
    }

    fn title(&self) -> String {
        lib_locale::msg::nav_reports()
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    #[test]
    fn renders_without_panicking() {
        let view = ReportsView::new();
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| view.view(frame, frame.area()))
            .expect("rendering the placeholder Reports view should not error");
    }

    #[test]
    fn title_is_the_reports_message() {
        crate::locale::init_for_tests();
        assert_eq!(ReportsView::new().title(), "Reports");
        assert_eq!(ReportsView::new().id(), ViewId::Reports);
    }
}
