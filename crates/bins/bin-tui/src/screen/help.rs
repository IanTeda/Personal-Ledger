//! The Help screen — "Decide keybinding and navigation/workflow scheme" named `?` as a
//! global key; this is where it leads, reachable from any screen via `App`'s global
//! dispatch (`docs/adr/0003-...`'s hybrid architecture routes it the same as any other
//! [`Action`]).

use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Paragraph},
};

use crate::{action::Action, screen::Screen};

/// Lists the keys that work from anywhere, plus the baseline vocabulary every list screen
/// will share once entity screens land.
pub struct HelpScreen;

impl HelpScreen {
    pub fn new() -> Self {
        Self
    }
}

impl Default for HelpScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for HelpScreen {
    fn update(&mut self, _action: &Action) {}

    fn title(&self) -> &'static str {
        "Help"
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        let paragraph = Paragraph::new(
            "Global\n  \
             Ctrl+C   hard quit, from anywhere\n  \
             Esc      back one level (quits at the Dashboard)\n  \
             ?        this screen\n\n\
             List screens (once built)\n  \
             ↑/k ↓/j  move\n  \
             Enter    open\n  \
             n        new\n  \
             d        delete (press y to confirm)\n  \
             /        filter\n  \
             s        cycle sort\n\n\
             Esc: back",
        )
        .block(Block::bordered().title(" Help "));
        frame.render_widget(paragraph, area);
    }
}
