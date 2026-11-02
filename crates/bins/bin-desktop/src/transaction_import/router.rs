//! `impl ImportRouter` for the Import step. Its Router records a close for the page to read, and
//! puts a Notice onto the bin's queue (`crate::notices`).

use lib_toast::{Notice, Notify};
use lib_transaction_import::{ImportCommitted, ImportRouter};

/// Where the step's Presenter asks to go: closing, which the page reads, and Notices, which `Root`
/// drains once the key or click returns.
#[derive(Default)]
pub(super) struct PageRouter {
    /// `Some` once the Presenter left the step, holding what its commit wrote or `None` when it
    /// was cancelled. The nested `Option` is the close every Desktop Router records (the Dialogs'
    /// `closed`, `bill_payments::router`), kept so this one reads like them rather than as an enum
    /// of its own.
    pub closed: Option<Option<ImportCommitted>>,
}

impl ImportRouter for PageRouter {
    fn close_import(&mut self, committed: Option<ImportCommitted>) {
        self.closed = Some(committed);
    }
}

impl Notify for PageRouter {
    fn notify(&mut self, notice: Notice) {
        crate::notices::Queue.notify(notice);
    }
}
