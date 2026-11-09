//! The Documents page's Dialogs, each its `lib-documents` Presenter in a `ModuleDialog`
//! (ADR-0056): Add and Edit, Import, Edit extracted facts, Accept all, Delete document and the link
//! picker. Each file is one Presenter's `DialogView`: how it draws, names and reads back its view
//! model, and the widget state gpui keeps beside it (a select's list window).
//!
//! A Presenter closes its Dialog through the host's Router with what the page does next, which
//! lands on the page that opened it inside the closing input. A Toast or status message the
//! Presenter raises goes onto the bin's queue, which the input's `send` drains.

mod accept_all;
mod delete_document;
mod document_form;
mod facts;
mod import;
mod picker;

use gpui::Context;
use lib_documents::presenter::{
    AcceptAllPresenter, DeleteDocumentPresenter, DialogPorts, DocumentFormPresenter,
    FactsFormPresenter, ImportPresenter, LinkPickerPresenter, Purpose,
};
use lib_documents::{DocumentsDialog as Which, DocumentsOutcome};
use lib_viper::dialog::DialogKeys;

pub(crate) use picker::snapshot as picker_snapshot;

use super::page::DocumentsPage;
use crate::shell::view::dialog_entity::{DialogView, ModuleDialog};

/// Opens `which`'s Dialog over the page on the page's `ports`, its hints under `keys`. Returns
/// `false`, opening nothing, when the Document it names is gone. A confirm's outcome lands on the
/// page through [`DocumentsPage::landing`].
pub(crate) fn open(
    which: Which,
    ports: DialogPorts,
    keys: DialogKeys,
    cx: &mut Context<'_, DocumentsPage>,
) -> bool {
    match which {
        Which::Add(scope) => host(cx, Some(DocumentFormPresenter::add(ports, scope, keys))),
        Which::Edit(id) => host(cx, DocumentFormPresenter::edit(ports, id, keys)),
        Which::Import => host(cx, Some(ImportPresenter::new(ports, keys))),
        Which::Facts(id) => host(cx, FactsFormPresenter::new(ports, id, keys)),
        Which::AcceptAll(strong) => host(cx, Some(AcceptAllPresenter::new(strong, keys))),
        Which::Delete(id) => host(cx, DeleteDocumentPresenter::new(ports, id, keys)),
        Which::Link(id) => host(cx, LinkPickerPresenter::new(ports, Purpose::Link(id), keys)),
        Which::File(id) => host(cx, LinkPickerPresenter::new(ports, Purpose::File(id), keys)),
        Which::Follow(id) => host(
            cx,
            LinkPickerPresenter::new(ports, Purpose::Follow(id), keys),
        ),
    }
}

/// Hosts `presenter`'s Dialog over the page, if there is one to host.
fn host<P: DialogView<Outcome = DocumentsOutcome>>(
    cx: &mut Context<'_, DocumentsPage>,
    presenter: Option<P>,
) -> bool {
    let Some(presenter) = presenter else {
        return false;
    };
    ModuleDialog::open(cx, presenter, DocumentsPage::landing(cx));
    true
}
