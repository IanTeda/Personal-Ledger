//! `impl DocumentTypesRouter` for the Desktop: where the Document Types page's Presenter asks to
//! go. A Dialog opens as an Entity in the Dialog host through the page's gpui context, which is
//! why the Router is built for each `handle` call, and a Notice goes onto the bin's queue
//! (`crate::notices`), which the input's `send` drains.

use gpui::{Context, Entity};
use lib_documents::{
    DocumentTypesDialog as Which, DocumentTypesOutcome, DocumentTypesRouter, SharedRepository,
};
use lib_toast::{Notice, Notify};

use super::DocumentTypesSetup;
use super::view::dialog::DocumentTypesDialog;
use super::view::page::DocumentTypesPage;
use crate::ledger_store::LedgerStore;
use crate::notices::Queue;
use crate::shell::view::dialog_entity::host_dialog;

pub(super) struct PageRouter<'a, 'b> {
    pub documents: SharedRepository,
    pub ledger: Entity<LedgerStore>,
    pub cx: &'a mut Context<'b, DocumentTypesPage>,
}

impl DocumentTypesRouter for PageRouter<'_, '_> {
    /// Builds `dialog`'s Presenter on the shared repository and hosts its View; its confirmed
    /// outcome comes back to the page through [`DocumentTypesPage::dialog_landed`].
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let page = self.cx.weak_entity();
        let landed = Box::new(move |outcome, cx: &mut gpui::App| {
            // A page that has gone has nothing to land on.
            let _ = page.update(cx, |page, cx| page.dialog_landed(outcome, cx));
        });
        let Some(dialog) = DocumentTypesDialog::open(dialog, &self.documents, &self.ledger, landed)
        else {
            return false;
        };
        host_dialog(self.cx, |_| dialog);
        true
    }
}

impl Notify for PageRouter<'_, '_> {
    /// Queues `notice`, for the input's `send` to drain into the Shell.
    fn notify(&mut self, notice: Notice) {
        Queue.notify(notice);
    }
}

/// The Document Types module's entry point: its Settings page, built on `setup`'s store, which
/// the Settings View hosts as any other page.
pub fn document_types_page(
    setup: DocumentTypesSetup,
    ledger: Entity<LedgerStore>,
    cx: &mut gpui::App,
) -> Entity<DocumentTypesPage> {
    use gpui::AppContext as _;
    cx.new(|cx| DocumentTypesPage::new(setup, ledger, cx))
}
