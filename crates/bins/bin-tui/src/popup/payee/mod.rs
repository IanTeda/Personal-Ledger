//! Payee-domain popups: floating overlays for creating, editing, matching and deleting a
//! `Payee` (`docs/ux/tui/payees/README.md` "8b"/"8c"/"8d"/"8e"), hosted by `Shell` the same way
//! `crate::popup::account`/`crate::popup::tag` are. `new` (8b), `edit` (8c) and `matches` (8d)
//! exist so far.

pub mod edit;
pub mod matches;
pub mod new;

use ratatui::{Frame, layout::Rect};

use crate::payee::PayeeStore;

/// The one Payee-domain popup `Shell` can have open at a time — mirrors
/// `crate::popup::account::AccountPopup`'s own single-`Option`-of-an-enum shape.
pub enum PayeePopup {
    New(new::NewPayeePopup),
    Edit(edit::EditPayeePopup),
    Matches(matches::PayeeMatchesPopup),
}

impl PayeePopup {
    /// Renders whichever form is open, against the live Payee list `store` — `new`'s
    /// name-collision check, `edit`'s rename preview, and `matches`'s conflict/resolution
    /// previews all need read access to it.
    pub fn render(&self, frame: &mut Frame, area: Rect, store: &dyn PayeeStore) {
        match self {
            PayeePopup::New(popup) => popup.render(frame, area, store),
            PayeePopup::Edit(popup) => popup.render(frame, area, store),
            PayeePopup::Matches(popup) => popup.render(frame, area, store),
        }
    }
}
