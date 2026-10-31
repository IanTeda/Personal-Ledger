//! What the integration tests read back of the Transactions list and its Import step. Each
//! Entity builds its own (ADR-0037); `Root`'s accessors forward to them.

use crate::shell::router::Root;
#[doc(hidden)]
pub use crate::transaction_import::view::snapshot::{ImportRowSnapshot, ImportSnapshot};
#[doc(hidden)]
pub use crate::transactions::view::snapshot::{ChipSnapshot, TransactionsSnapshot};

impl Root {
    /// The Transactions page for a test, as its Entity builds it.
    #[doc(hidden)]
    pub fn transactions_snapshot(&self, cx: &gpui::App) -> TransactionsSnapshot {
        self.views.transactions.read(cx).snapshot(cx)
    }

    /// The Import step for a test; `None` while it is not showing.
    #[doc(hidden)]
    pub fn import_snapshot(&self, cx: &gpui::App) -> Option<ImportSnapshot> {
        self.views.import.read(cx).snapshot()
    }
}
