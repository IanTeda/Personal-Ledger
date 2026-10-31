//! The Transactions destination's cross-cutting wiring: the rows other pages read and `:import`.
//! Hand-offs into the page go through `crate::transactions::router`, and the page itself is the
//! Transactions module's (`crate::transactions`).

use super::Root;
use lib_shell::NavInput;

use crate::{
    navigation::nav::{FocusZone, Noun},
    transactions::Transaction,
};
use gpui::App;

impl Root {
    /// The Transactions, read through their store (ADR-0032).
    pub(super) fn transactions<'a>(&self, cx: &'a App) -> &'a [Transaction] {
        self.ledger.read(cx).ledger().transactions()
    }

    /// `:import`: opens the stubbed 6e step on the seeded statement, the Import step shown in place
    /// of the Transactions list (#284, #924, #947).
    pub(super) fn open_import(&mut self, cx: &mut App) {
        self.navigate(NavInput::Show(Noun::Transactions));
        self.navigate(NavInput::Focus(FocusZone::View));
        self.reset_view_scroll(cx);
        self.views.import.update(cx, |view, cx| view.open(cx));
    }
}
