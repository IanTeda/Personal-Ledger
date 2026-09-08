//! The Categories `View`, hosted by `Shell` (ADR-0013). Wireframe stage: a single bordered
//! box proving the navigation path — the command palette's `category list` entry and the
//! global `g c` jump chord both land here — before any real list/detail content is built out.

use ratatui::{Frame, layout::Rect, widgets::Block};

use crate::view::{Action, View};

/// A trivial placeholder Categories `View`: one bordered box, no content yet.
#[derive(Default)]
pub struct CategoriesView;

impl CategoriesView {
    pub fn new() -> Self {
        Self
    }
}

impl View for CategoriesView {
    fn update(&mut self, _action: &Action) {}

    fn view(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Block::bordered().title(" Categories "), area);
    }

    fn title(&self) -> &'static str {
        "Categories"
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    #[test]
    fn renders_without_panicking() {
        let view = CategoriesView::new();
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| view.view(frame, frame.area()))
            .expect("rendering the placeholder Categories view should not error");
    }

    #[test]
    fn title_is_categories() {
        assert_eq!(CategoriesView::new().title(), "Categories");
    }
}
