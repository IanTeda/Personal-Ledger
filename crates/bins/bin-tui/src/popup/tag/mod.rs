//! Tag-domain popups: floating overlays for creating and editing a `Tag`
//! (issue #125, "Tags catalog screen, views and popup"), hosted by `Shell` the same way
//! `crate::popup::unit`/`crate::popup::category`/`crate::popup::account` are. `new` (issue
//! #128) and `edit` (issue #129) exist so far, both genuinely interactive and genuinely
//! mutating the fixture, mirroring `crate::popup::account`'s own `new`/`edit`. Tag's delete is
//! the lightweight arm-and-confirm built directly into `view::tags::TagsView`, not a popup
//! here — see `edit`'s own module doc for why there is no `Delete` variant.

pub mod edit;
pub mod new;

use ratatui::{Frame, layout::Rect};

use crate::tag::TagStore;

/// The one Tag-domain popup `Shell` can have open at a time — mirrors
/// `crate::popup::account::AccountPopup`'s own single-`Option`-of-an-enum shape.
pub enum TagPopup {
    New(new::NewTagPopup),
    Edit(edit::EditTagPopup),
}

impl TagPopup {
    /// Renders whichever form is open, against the live Tag list `store` — `new`'s and
    /// `edit`'s uniqueness validation both need read access to it, mirroring
    /// `AccountPopup::render`.
    pub fn render(&self, frame: &mut Frame<'_>, area: Rect, store: &dyn TagStore) {
        match self {
            TagPopup::New(popup) => popup.render(frame, area, store),
            TagPopup::Edit(popup) => popup.render(frame, area, store),
        }
    }
}
