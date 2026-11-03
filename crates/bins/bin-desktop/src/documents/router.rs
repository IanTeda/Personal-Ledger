//! `impl DocumentsRouter` for the Desktop: where the Documents page's Presenter asks to go. A Link
//! is revealed through the View that owns its record (Transactions through its entry point, the
//! rest as a `Reveal` the Shell routes), a file goes to the platform, a Dialog opens as an Entity
//! in the Dialog host through the page's gpui context, which is why the Router is built for each
//! `handle` call, and a Notice goes onto the bin's queue (`crate::notices`), which the input's
//! `send` drains.

use std::path::Path;

use gpui::{App, Context};
use lib_documents::presenter::DialogPorts;
use lib_documents::{DocumentLink, DocumentsDialog as Which, DocumentsOutcome, DocumentsRouter};
use lib_toast::{Notice, Notify};

use super::view::dialog::DocumentsDialog;
use super::view::page::DocumentsPage;
use crate::navigation::nav::Noun;
use crate::notices::Queue;
use crate::shell::view::dialog_entity::host_dialog;
use crate::view::event::{Reveal, ViewEvent};

pub(super) struct PageRouter<'a, 'b> {
    /// What the page's Dialogs read and write through.
    pub ports: DialogPorts,
    pub cx: &'a mut Context<'b, DocumentsPage>,
}

impl DocumentsRouter for PageRouter<'_, '_> {
    /// A Transaction through its own entry point; an Account, Payee or Bill Plan as a `Reveal` the
    /// Shell routes; an Inventory Item shows the Inventory, which has no record reveal yet. There
    /// is no back-history: `g f` returns to Documents.
    fn reveal(&mut self, link: DocumentLink) {
        let event = match link {
            DocumentLink::Transaction(id) => {
                crate::transactions::router::show(self.cx, id);
                return;
            }
            DocumentLink::Account(id) => ViewEvent::Reveal(Reveal::Account(id)),
            DocumentLink::Payee(id) => ViewEvent::Reveal(Reveal::Payee(id)),
            DocumentLink::BillPlan(id) => ViewEvent::Reveal(Reveal::BillPlan(id)),
            DocumentLink::InventoryItem(_) => ViewEvent::Navigate(Noun::Inventory),
        };
        self.cx.emit(event);
    }

    /// Builds `dialog`'s Presenter on the page's ports and hosts its View; what it asks of the page
    /// as it closes comes back through [`DocumentsPage::dialog_landed`].
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let page = self.cx.weak_entity();
        let landed = Box::new(move |outcome, cx: &mut App| {
            // A page that has gone has nothing to land on.
            let _ = page.update(cx, |page, cx| page.dialog_landed(outcome, cx));
        });
        let Some(dialog) = DocumentsDialog::open(dialog, self.ports.clone(), landed) else {
            return false;
        };
        host_dialog(self.cx, |_| dialog);
        true
    }

    fn open_file(&mut self, path: &Path, in_folder: bool) {
        let app: &mut App = self.cx;
        if in_folder {
            app.reveal_path(path);
        } else {
            app.open_with_system(path);
        }
    }
}

impl Notify for PageRouter<'_, '_> {
    /// Queues `notice`, for the input's `send` to drain into the Shell.
    fn notify(&mut self, notice: Notice) {
        Queue.notify(notice);
    }
}
