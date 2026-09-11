//! Category-domain popups: floating overlays acting on the Category tree
//! (`docs/ux/tui/categories/README.md`), hosted by `Shell` the same way `crate::popup::unit`
//! is. `move_popup` (5b) exists so far; `new`/`edit` (5c/5d) join this enum in their own
//! tickets ("Categories: 5c new popup", "Categories: 5d edit popup").
//!
//! Unlike `popup::unit`'s forms (still wireframe-only, "no draft state yet"), this popup is
//! genuinely interactive and genuinely mutates the tree — see `move_popup`'s own module doc
//! for how it reaches `CategoriesView`'s store despite `Shell` not owning it.

pub mod move_popup;

use ratatui::{Frame, layout::Rect};

use crate::category::CategoryStore;

/// The one Category-domain popup `Shell` can have open at a time — mirrors
/// `crate::popup::unit::UnitPopup`'s own single-`Option`-of-an-enum shape.
pub enum CategoryPopup {
    Move(move_popup::MovePopup),
}

impl CategoryPopup {
    /// Renders whichever form is open, against the live Category tree `store` — see
    /// `move_popup::MovePopup::render` for why this needs read access to it.
    pub fn render(&self, frame: &mut Frame, area: Rect, store: &dyn CategoryStore) {
        match self {
            CategoryPopup::Move(popup) => popup.render(frame, area, store),
        }
    }
}
