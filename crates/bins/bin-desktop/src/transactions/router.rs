//! The Transactions module's entry points for other modules' Routers. The Transactions View isn't
//! a VIPER module yet, so [`show`] hands off the way every View does today, a
//! [`Reveal::Transaction`] the Shell routes to the Transactions page (#778's stand-in until that
//! View migrates).

use gpui::{Context, EventEmitter};
use lib_core::TransactionId;

use crate::view::event::{Reveal, ViewEvent};

/// Shows Transaction `id` on the Transactions page, selected, from the View `cx` belongs to.
pub fn show<T: EventEmitter<ViewEvent>>(cx: &mut Context<'_, T>, id: TransactionId) {
    cx.emit(ViewEvent::Reveal(Reveal::Transaction(id)));
}
