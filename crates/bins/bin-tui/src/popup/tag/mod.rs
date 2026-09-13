//! Tag-domain popups: floating overlays for creating and editing a `Tag`
//! (issue #125, "Tags catalog screen, views and popup"), hosted by `Shell` the same way
//! `crate::popup::unit`/`crate::popup::category`/`crate::popup::account` are. `new` (issue
//! #128) exists so far, genuinely interactive and genuinely mutating the fixture, mirroring
//! `crate::popup::account`'s own `new`.

pub mod new;

use ratatui::{Frame, layout::Rect};

use crate::tag::TagStore;

/// The one Tag-domain popup `Shell` can have open at a time — mirrors
/// `crate::popup::account::AccountPopup`'s own single-`Option`-of-an-enum shape.
pub enum TagPopup {
    New(new::NewTagPopup),
}

impl TagPopup {
    /// Renders whichever form is open, against the live Tag list `store` — `new`'s
    /// uniqueness validation needs read access to it, mirroring `AccountPopup::render`.
    pub fn render(&self, frame: &mut Frame, area: Rect, store: &dyn TagStore) {
        match self {
            TagPopup::New(popup) => popup.render(frame, area, store),
        }
    }
}
