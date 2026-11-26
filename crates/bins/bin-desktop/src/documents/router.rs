//! `impl DocumentsRouter` for the Desktop: where the Documents page's Presenter asks to go. A Link
//! is revealed through the View that owns its record (Transactions through its entry point, the
//! rest as a `Reveal` the Shell routes), a file goes to the platform, a Dialog opens as an Entity
//! in the Dialog host through the generic [`PageRouter`]'s gpui context, which is why the Router is
//! built for each `handle` call, and it queues a Notice for the input's `send` to drain.

use std::path::Path;

use gpui::{App, AppContext as _, Entity};
use lib_core::Clock;
use lib_documents::presenter::{DialogPorts, DocumentsPagePresenter};
use lib_documents::{
    DocumentLink, DocumentsDialog as Which, DocumentsRouter, SharedFiles, SharedRepository,
};

use super::view::page::DocumentsPage;
use crate::announcer::Announcer;
use crate::navigation::nav::Noun;
use crate::shell::view::module_page::ModulePage;
use crate::shell::view::page_router::PageRouter;
use crate::view::event::{Reveal, ViewEvent};

/// The Documents page's Router over its Setup, the Dialog keymap's chords, which the Dialogs' hints
/// show, lent the ports the page's Dialogs read and write through: the Presenter's.
pub(crate) type DocumentsPageRouter<'a, 'b> =
    PageRouter<'a, 'b, DocumentsPagePresenter, DialogPorts>;

impl DocumentsRouter for DocumentsPageRouter<'_, '_> {
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

    /// Builds `dialog`'s Presenter on the page's ports and hosts it in a `ModuleDialog`; what it
    /// asks of the page as it closes comes back through [`DocumentsPage::landing`].
    fn open_dialog(&mut self, dialog: Which) -> bool {
        super::view::dialog::open(dialog, self.lent.clone(), self.setup.get(), self.cx)
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

/// The Documents module's entry point: its page, on the Documents repository (`documents`), the other
/// domains' reads over their stores (`reads`) and the disk (`files`), which the Shell hosts as the
/// Documents destination. `clock` reads the day the Key Dates are banded against. The page writes
/// the store through its Interactors; the input that wrote announces the Documents, so the rail's
/// Inbox count follows a filing. Its keys, and the Dialog keys its Dialogs' hints show, are
/// `config`'s, read from `dialog_keys` as each Dialog opens so a rebind reaches it.
#[expect(
    clippy::too_many_arguments,
    reason = "the page's Setup is the shared Dialog keys alone; the rest build its Presenter"
)]
pub fn documents_page(
    documents: &SharedRepository,
    files: SharedFiles,
    reads: lib_documents::SharedReads,
    announcer: Entity<Announcer>,
    config: &lib_core::configuration::KeyBindingConfig,
    clock: Clock,
    dialog_keys: crate::dialog_keys::LiveDialogKeys,
    cx: &mut App,
) -> Entity<DocumentsPage> {
    let documents = documents.clone();
    let presenter = DocumentsPagePresenter::new(
        documents,
        reads,
        files,
        clock,
        config,
        crate::shell::view::statusline::STUB_LEDGER_FILE.to_string(),
    );
    cx.new(|cx| ModulePage::new(presenter, dialog_keys, &announcer, cx))
}
