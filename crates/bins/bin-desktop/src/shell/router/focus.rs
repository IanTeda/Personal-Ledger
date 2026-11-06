//! The view-scroll reset of `Root`. An `impl Root` block for the concern, not the focus model
//! itself (`navigation::nav`): the Shell's Presenter moves focus and the rails, and the root Router
//! the View's part of a movement.

use super::{Root, root_router};

impl Root {
    /// The active View has just become the destination: its scroll goes back to the top
    /// ([`root_router::reset_view_scroll`]).
    pub(super) fn reset_view_scroll(&mut self, cx: &mut gpui::App) {
        let active = self.active_view_entity(cx);
        root_router::reset_view_scroll(&self.view_x_scroll_handle, active, self.open.as_ref(), cx);
    }
}
