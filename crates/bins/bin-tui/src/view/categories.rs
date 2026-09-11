//! The Categories `View`, hosted by `Shell` (ADR-0013). Rendering is still wireframe stage: a
//! single bordered box proving the navigation path — the command popup's `category list`
//! entry and the global `g c` jump chord both land here — before the real tree/summary/chart/
//! transactions content is built out ("Categories: 5a screen — tree pane and summary box
//! (left pane)" onward). It already owns real, mutable state: the `CategoryFixture` seeded
//! tree from `crate::category`, per that map's "in-memory mutable fixture" decision — later
//! tickets build the rendering and key handling on top of what's already wired here.

use ratatui::{Frame, layout::Rect, widgets::Block};

use crate::category::CategoryFixture;
use crate::view::{Action, View};

/// The Categories `View`. Owns the fixture Category tree directly — no navigation stack
/// action carries it, per `crate::category`'s state-ownership decision — so a future screen's
/// `handle_key` can mutate `store` in place (fold, new, move, edit) without any `Action`
/// plumbing beyond what already signals opening/closing this `View` and its popups.
pub struct CategoriesView {
    store: CategoryFixture,
}

impl Default for CategoriesView {
    fn default() -> Self {
        Self::new()
    }
}

impl CategoriesView {
    pub fn new() -> Self {
        Self {
            store: CategoryFixture::new(),
        }
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
    use crate::category::CategoryStore;

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

    #[test]
    fn owns_a_seeded_fixture_tree_with_both_roots() {
        let view = CategoriesView::new();
        let roots: Vec<_> = view
            .store
            .nodes()
            .iter()
            .filter(|node| node.parent_id.is_none())
            .collect();
        assert_eq!(roots.len(), 2);
    }
}
