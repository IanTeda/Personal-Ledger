//! The Transactions module's entry points for other modules' Routers. Halfway (#923): the page is
//! a module, but [`show`] still hands off the way every View does today, a
//! [`Reveal::Transaction`] the Shell routes to the page, which turns it into a Presenter event.
//! The next ticket makes this the one entry point every caller uses.

use gpui::{Context, EventEmitter};
use lib_core::TransactionId;

use crate::view::event::{Reveal, ViewEvent};

/// Shows Transaction `id` on the Transactions page, selected, from the View `cx` belongs to.
pub fn show<T: EventEmitter<ViewEvent>>(cx: &mut Context<'_, T>, id: TransactionId) {
    cx.emit(ViewEvent::Reveal(Reveal::Transaction(id)));
}
