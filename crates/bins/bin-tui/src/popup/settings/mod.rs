//! Settings-domain popups: the in-place editor (`docs/ux/tui/settings/README.md` §4b) and the
//! base-unit guard (§4c), hosted by `Shell` the same way `crate::popup::unit` is. Wireframe
//! stage, same fidelity as the rest of `view::settings` and `popup::unit`: every field renders
//! §4a/§4b/§4c's own worked examples verbatim rather than a real, editable draft against the
//! (not yet built) settings registry.
//!
//! §4b's own text calls for the editor to open "in the row's position" — the list dims around
//! it rather than a floating dialog. Implementing that would mean the settings list's row
//! layout knowing about an open editor, which is a deeper change than this wireframe stage
//! needs; both popups here instead reuse `popup::unit`'s "centred floating overlay over a
//! dimmed view" treatment, the same simplification the units screen's own forms already made
//! against their design doc's more contextual language.

pub mod edit;
pub mod guard;

use ratatui::{Frame, layout::Rect};

/// The one settings-domain popup `Shell` can have open at a time — mirrors `popup::unit::
/// UnitPopup`'s own single-`Option` reasoning: only one form is ever up.
pub enum SettingsPopup {
    Edit(edit::EditSettingPopup),
    BaseUnitGuard(guard::BaseUnitGuardPopup),
}

impl SettingsPopup {
    /// Renders whichever popup is open — see each variant's own `render` for its layout.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        match self {
            SettingsPopup::Edit(popup) => popup.render(frame, area),
            SettingsPopup::BaseUnitGuard(popup) => popup.render(frame, area),
        }
    }
}
