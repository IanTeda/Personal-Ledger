//! The Settings drill-in — "Decide the application Settings / Preference model" locked in a
//! single V1 Preference (the Ledger-scoped default Unit for new Accounts), shown here as an
//! in-place editable field once Units exist (issue #67) to pick from.

use ratatui::{
    Frame,
    layout::Rect,
    widgets::{Block, Paragraph},
};

use crate::{action::Action, screen::Screen};

/// The application's Settings screen — one field for V1.
pub struct SettingsScreen;

impl SettingsScreen {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SettingsScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl Screen for SettingsScreen {
    fn update(&mut self, _action: &Action) {}

    fn title(&self) -> &'static str {
        "Settings"
    }

    fn view(&self, frame: &mut Frame, area: Rect) {
        let paragraph = Paragraph::new(
            "Default Unit for new Accounts: (none yet — build Units, issue #67, to pick one)\n\n\
             Esc: back",
        )
        .block(Block::bordered().title(" Settings "));
        frame.render_widget(paragraph, area);
    }
}
