//! Settings-domain popups and the in-place editor. `Shell` hosts the colour list popups and the
//! base unit guard (TUI-021) over the dimmed Settings View; the in-place editor (TUI-020) isn't a
//! popup, but a box the form page draws in the open field's row, so the Settings View holds it.
//! The Edit overlays of Sync Server, Backup and AI Connections (TUI-035, TUI-036, TUI-034) are the
//! Settings View's too, as its pages' own: three popups on one frame ([`overlay`]).

pub mod colour;
pub mod edit;
pub mod git_backup;
pub mod guard;
pub mod overlay;
pub mod provider;
pub mod sync_server;

use crate::colours::Colours;
use ratatui::{Frame, layout::Rect};

/// The one settings-domain popup `Shell` can have open at a time.
pub enum SettingsPopup {
    BaseUnitGuard(guard::BaseUnitGuardPopup),
    Colour(colour::ColourPopup),
}

impl SettingsPopup {
    /// Renders whichever popup is open — see each variant's own `render` for its layout.
    pub fn render(&self, frame: &mut Frame<'_>, area: Rect, c: &Colours) {
        match self {
            SettingsPopup::BaseUnitGuard(popup) => popup.render(frame, area, c),
            SettingsPopup::Colour(popup) => popup.render(frame, area, c),
        }
    }
}
