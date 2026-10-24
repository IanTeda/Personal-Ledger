//! What the integration tests read back of the Transactions page and its Import step. The
//! Transactions Entity builds both (ADR-0037); `Root`'s accessors forward to it.

use crate::shell::router::Root;
#[doc(hidden)]
pub use crate::view::transactions::snapshot::{
    ChipSnapshot, ImportRowSnapshot, ImportSnapshot, TransactionsSnapshot,
};

impl Root {
    /// The Transactions page for a test, as its Entity builds it.
    #[doc(hidden)]
    pub fn transactions_snapshot(&self, cx: &gpui::App) -> TransactionsSnapshot {
        self.views.transactions.read(cx).snapshot(cx)
    }

    /// The Import step for a test; `None` while it is not showing.
    #[doc(hidden)]
    pub fn import_snapshot(&self, cx: &gpui::App) -> Option<ImportSnapshot> {
        self.views.transactions.read(cx).import_snapshot()
    }
}
