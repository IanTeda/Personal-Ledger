//! The Help `View`, hosted by `Shell` (ADR-0013). Wireframe stage: a single bordered box
//! proving the navigation path — the command popup's `help` entry and the global `?` key
//! both land here — before the real `:help` browse-every-command content is built out.

use ratatui::{Frame, layout::Rect, widgets::Block};

use crate::view::{Action, View, ViewId};

/// A trivial placeholder Help `View`: one bordered box, no content yet.
#[derive(Default)]
pub struct HelpView;

impl HelpView {
    pub fn new() -> Self {
        Self
    }
}

impl View for HelpView {
    fn update(&mut self, _action: &Action) {}

    fn view(&self, frame: &mut Frame, area: Rect) {
        frame.render_widget(Block::bordered().title(format!(" {} ", self.title())), area);
    }

    fn id(&self) -> ViewId {
        ViewId::Help
    }

    fn title(&self) -> String {
        lib_locale::msg::nav_help()
    }
}

#[cfg(test)]
mod tests {
    use ratatui::{Terminal, backend::TestBackend};

    use super::*;

    #[test]
    fn renders_without_panicking() {
        let view = HelpView::new();
        let backend = TestBackend::new(96, 30);
        let mut terminal = Terminal::new(backend).expect("test backend should initialise");

        terminal
            .draw(|frame| view.view(frame, frame.area()))
            .expect("rendering the placeholder Help view should not error");
    }

    #[test]
    fn title_is_help() {
        crate::locale::init_for_tests();
        assert_eq!(HelpView::new().title(), "Help");
        assert_eq!(HelpView::new().id(), ViewId::Help);
    }
}
