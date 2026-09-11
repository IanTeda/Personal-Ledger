//! Category-domain popups: floating overlays acting on the Category tree
//! (`docs/ux/tui/categories/README.md`), hosted by `Shell` the same way `crate::popup::unit`
//! is. `move_popup` (5b) and `new_popup` (5c) exist so far; `edit` (5d) joins this enum in its
//! own ticket ("Categories: 5d edit popup").
//!
//! Unlike `popup::unit`'s forms (still wireframe-only, "no draft state yet"), these popups are
//! genuinely interactive and genuinely mutate the tree — see `move_popup`'s own module doc for
//! how they reach `CategoriesView`'s store despite `Shell` not owning it. `path` is the
//! `/`-separated path resolution both `move_popup`'s `new parent` and `new_popup`'s `parent`
//! fields share.

pub mod move_popup;
pub mod new_popup;
pub mod path;

use ratatui::{Frame, layout::Rect};

use crate::category::CategoryStore;

/// The one Category-domain popup `Shell` can have open at a time — mirrors
/// `crate::popup::unit::UnitPopup`'s own single-`Option`-of-an-enum shape.
pub enum CategoryPopup {
    Move(move_popup::MovePopup),
    New(new_popup::NewPopup),
}

impl CategoryPopup {
    /// Renders whichever form is open, against the live Category tree `store` — see
    /// `move_popup::MovePopup::render` for why this needs read access to it.
    pub fn render(&self, frame: &mut Frame, area: Rect, store: &dyn CategoryStore) {
        match self {
            CategoryPopup::Move(popup) => popup.render(frame, area, store),
            CategoryPopup::New(popup) => popup.render(frame, area, store),
        }
    }
}
