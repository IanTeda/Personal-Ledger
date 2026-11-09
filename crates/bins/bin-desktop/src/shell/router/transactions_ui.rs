//! The Transactions destination's cross-cutting wiring: `:import`.
//! Hand-offs into the page go through `crate::transactions::router`, and the page itself is the
//! Transactions module's (`crate::transactions`).

use super::Root;
use lib_shell::NavInput;

use crate::navigation::nav::{FocusZone, Noun};
use gpui::App;

impl Root {
    /// `:import`: opens the stubbed 6e step on the seeded statement, the Import step shown in place
    /// of the Transactions list (#284, #924, #947).
    pub(super) fn open_import(&mut self, cx: &mut App) {
        self.navigate(NavInput::Show(Noun::Transactions));
        self.navigate(NavInput::Focus(FocusZone::View));
        self.reset_view_scroll(cx);
        if let Some(open) = &self.open {
            open.views.import.update(cx, |view, cx| {
                view.send(lib_transaction_import::presenter::ImportEvent::OpenStep, cx);
            });
        }
    }
}
