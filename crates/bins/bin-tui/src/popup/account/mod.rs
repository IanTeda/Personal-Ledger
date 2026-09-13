//! Account-domain popups: floating overlays for creating, editing and deleting an `Account`
//! (`docs/ux/tui/accounts/README.md` "7b"/"7c"/"7d"), hosted by `Shell` the same way
//! `crate::popup::unit`/`crate::popup::category` are. `new` (7b) exists so far — unlike
//! `popup::unit`'s own forms ("no draft state yet"), it's genuinely interactive and genuinely
//! mutates the fixture, mirroring `popup::category`'s own new/edit/move popups.

pub mod delete;
pub mod edit;
pub mod new;

use ratatui::{Frame, layout::Rect};

use crate::account::AccountStore;

/// The one Account-domain popup `Shell` can have open at a time — mirrors
/// `crate::popup::category::CategoryPopup`'s own single-`Option`-of-an-enum shape.
pub enum AccountPopup {
    New(new::NewAccountPopup),
    Edit(edit::EditAccountPopup),
    Delete(delete::DeleteAccountPopup),
}

impl AccountPopup {
    /// Renders whichever form is open, against the live Account list `store` — `new`'s `unit`
    /// field completion, `edit`'s computed section, and `delete`'s transfer-candidate/preview
    /// all need read access to it.
    pub fn render(&self, frame: &mut Frame, area: Rect, store: &dyn AccountStore) {
        match self {
            AccountPopup::New(popup) => popup.render(frame, area, store),
            AccountPopup::Edit(popup) => popup.render(frame, area, store),
            AccountPopup::Delete(popup) => popup.render(frame, area, store),
        }
    }
}
