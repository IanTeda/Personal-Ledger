//! `impl ImportRouter` for the Import step: where its Presenter asks to go, through the page's gpui
//! context, which is why the Router is built for each `handle` call (`PageView::with_router`). A
//! commit lands on the Transactions list's first row, inside the same input; a Notice goes onto the
//! bin's queue (`crate::notices`), which the input's `send` drains.

use gpui::Context;
use lib_toast::{Notice, Notify};
use lib_transaction_import::{ImportCommitted, ImportRouter};

use super::ImportPage;

pub(super) struct PageRouter<'a, 'b> {
    pub cx: &'a mut Context<'b, ImportPage>,
}

impl ImportRouter for PageRouter<'_, '_> {
    /// The step has closed itself; a commit puts the Transactions list back on its first row.
    fn close_import(&mut self, committed: Option<ImportCommitted>) {
        if committed.is_some() {
            crate::transactions::router::first_row(self.cx);
        }
    }
}

impl Notify for PageRouter<'_, '_> {
    fn notify(&mut self, notice: Notice) {
        crate::notices::Queue.notify(notice);
    }
}
