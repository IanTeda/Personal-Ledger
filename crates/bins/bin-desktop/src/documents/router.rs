//! `impl DocumentsRouter` for the Desktop: where the Documents page's Presenter asks to go. A Link
//! is revealed through the View that owns its record (Transactions through its entry point, the
//! rest as a `Reveal` the Shell routes), a file goes to the platform, and a Notice goes onto the
//! bin's queue (`crate::notices`), which the input's `send` drains.
//!
//! The Dialogs are still the legacy host's (#968): the Router builds each one's form from the
//! Ledger read copy and asks the Shell to open it. A confirmed one writes through the Ledger's
//! forwarding shim and hands its outcome back to the page (`shell/router/documents_ui.rs`).

use std::path::Path;

use gpui::{App, Context, Entity};
use lib_core::{Clock, DateStyle};
use lib_documents::{DocumentLink, DocumentsDialog, DocumentsRouter, KeyDateKind};
use lib_toast::{Notice, Notify};

use super::form::{self, DocumentForm, DocumentOptions, FactsForm, ImportForm};
use super::picker::{PickerState, Purpose};
use super::view::page::DocumentsPage;
use crate::ledger_store::LedgerStore;
use crate::navigation::nav::Noun;
use crate::notices::Queue;
use crate::shell::view::dialog_host::OpenDialog;
use crate::view::event::{Reveal, ViewEvent};

pub(super) struct PageRouter<'a, 'b> {
    pub ledger: Entity<LedgerStore>,
    /// The day the Dialogs are opened on, and the date style their dates are written in.
    pub clock: Clock,
    pub date_style: Option<DateStyle>,
    pub cx: &'a mut Context<'b, DocumentsPage>,
}

impl PageRouter<'_, '_> {
    /// The Document Types' select labels, in the user's order.
    fn options(&self) -> DocumentOptions {
        DocumentOptions::new(
            self.ledger.read(self.cx).ledger().document_types(),
            key_label,
        )
    }

    fn document(&self, id: lib_core::DocumentId) -> Option<lib_documents::Document> {
        lib_documents::get(self.ledger.read(self.cx).ledger().documents(), id).cloned()
    }

    /// The link picker on `state`, offering every Document Type to file under.
    fn picker(&self, state: PickerState) -> form::DocumentsDialog {
        let types = self
            .ledger
            .read(self.cx)
            .ledger()
            .document_types()
            .iter()
            .map(|row| lib_documents::DocumentType(row.id))
            .collect();
        form::DocumentsDialog::Picker(Box::new(state.with_types(types)))
    }

    /// The legacy form `dialog` opens, built from the Ledger read copy; `None` when the Document it
    /// names has gone.
    fn build(&self, dialog: DocumentsDialog) -> Option<form::DocumentsDialog> {
        let today = self.clock.today();
        Some(match dialog {
            DocumentsDialog::Add(scope) => form::DocumentsDialog::Add(Box::new(DocumentForm::new(
                &self.options(),
                scope,
                today,
                self.date_style,
            ))),
            DocumentsDialog::Edit(id) => {
                let document = self.document(id)?;
                let form =
                    DocumentForm::for_edit(&document, &self.options(), today, self.date_style);
                form::DocumentsDialog::Edit(id, Box::new(form))
            }
            DocumentsDialog::Import => {
                form::DocumentsDialog::Import(ImportForm::default(), Vec::new())
            }
            DocumentsDialog::Facts(id) => {
                let facts = self
                    .document(id)?
                    .intake
                    .map(|intake| intake.facts)
                    .unwrap_or_default();
                let form = FactsForm::new(&facts, &self.options(), today, self.date_style);
                form::DocumentsDialog::Facts(id, Box::new(form))
            }
            DocumentsDialog::AcceptAll(count) => form::DocumentsDialog::AcceptAll(count),
            DocumentsDialog::Link(id) => {
                let document = self.document(id)?;
                self.picker(PickerState::new(Purpose::Link(id), document.date, None))
            }
            DocumentsDialog::File(id) => {
                let document = self.document(id)?;
                let (anchor, doc_type) =
                    document
                        .intake
                        .as_ref()
                        .map_or((document.date, None), |intake| {
                            (
                                intake.facts.date.unwrap_or(intake.received_at),
                                intake.facts.doc_type,
                            )
                        });
                self.picker(PickerState::new(Purpose::File(id), anchor, doc_type))
            }
            DocumentsDialog::Follow(id) => {
                let document = self.document(id)?;
                self.picker(PickerState::new(Purpose::Follow(id), document.date, None))
            }
        })
    }
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

    fn open_dialog(&mut self, dialog: DocumentsDialog) -> bool {
        let Some(dialog) = self.build(dialog) else {
            return false;
        };
        self.cx
            .emit(ViewEvent::OpenDialog(Box::new(OpenDialog::Documents(
                dialog,
            ))));
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

/// The Key date select's label for each kind.
fn key_label(kind: Option<KeyDateKind>) -> String {
    match kind {
        None => crate::msg::desktop_documents_key_none(),
        Some(KeyDateKind::Renews) => lib_documents::msg::documents_detail_key_renews(),
        Some(KeyDateKind::Ends) => lib_documents::msg::documents_detail_key_ends(),
        Some(KeyDateKind::Expires) => lib_documents::msg::documents_detail_key_expires(),
        Some(KeyDateKind::Revalue) => lib_documents::msg::documents_detail_key_revalue(),
    }
}
