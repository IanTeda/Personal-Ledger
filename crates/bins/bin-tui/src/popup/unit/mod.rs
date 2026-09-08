//! Unit-domain popups: floating overlays for creating, editing and deleting a `Unit`
//! (`docs/ux/tui/units/README.md` "The forms"), hosted by `Shell` the same way
//! `crate::popup::command` is. `new` (§4b), `edit` (§4c) and `delete` (§4e, the allowed
//! variant only — §4d's refused variant is later work) exist so far.

pub mod delete;
pub mod edit;
pub mod new;

use ratatui::{Frame, layout::Rect};

/// The one unit-domain popup `Shell` can have open at a time. Only one form is ever up, so
/// `Shell` holds a single `Option<UnitPopup>` rather than one `Option` per form (which would
/// need its own field, its own arm in every match over "which popup is open", and its own
/// mutual-exclusion bookkeeping against the others).
pub enum UnitPopup {
    New(new::NewUnitPopup),
    Edit(edit::EditUnitPopup),
    Delete(delete::DeleteUnitPopup),
}

impl UnitPopup {
    /// Renders whichever form is open — see each variant's own `render` for its layout.
    pub fn render(&self, frame: &mut Frame, area: Rect) {
        match self {
            UnitPopup::New(popup) => popup.render(frame, area),
            UnitPopup::Edit(popup) => popup.render(frame, area),
            UnitPopup::Delete(popup) => popup.render(frame, area),
        }
    }
}
