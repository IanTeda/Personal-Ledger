//! What the integration tests read back of the Documents page. The Documents Entity builds the
//! page's part; the open Dialog, the Toasts and the status message are `Root`'s, so they are
//! filled in here.

use lib_documents::presenter::{
    AcceptAllPresenter, DeleteDocumentPresenter, DocumentFormPresenter, FactsFormPresenter,
    ImportPresenter, LinkPickerPresenter,
};

use crate::documents::view::dialog::picker_snapshot;
#[doc(hidden)]
pub use crate::documents::view::snapshot::DocumentsSnapshot;
use crate::shell::router::Root;
use crate::shell::view::dialog_entity::ModuleDialog;

impl Root {
    /// The Documents page for a test, as its Entity builds it, plus the open Documents Dialog.
    #[doc(hidden)]
    pub fn documents_snapshot(&self, cx: &gpui::App) -> DocumentsSnapshot {
        let view = self.open().views.documents.read(cx);
        let mut snapshot = view.snapshot(cx);
        if let Some(open) = self.hosted_dialog() {
            let documents = open
                .downcast::<ModuleDialog<DocumentFormPresenter>>()
                .is_some()
                || open.downcast::<ModuleDialog<ImportPresenter>>().is_some()
                || open
                    .downcast::<ModuleDialog<FactsFormPresenter>>()
                    .is_some()
                || open
                    .downcast::<ModuleDialog<AcceptAllPresenter>>()
                    .is_some()
                || open
                    .downcast::<ModuleDialog<DeleteDocumentPresenter>>()
                    .is_some();
            let picker = open.downcast::<ModuleDialog<LinkPickerPresenter>>();
            if documents || picker.is_some() {
                snapshot.dialog = Some(open.readback(cx).kind);
            }
            snapshot.picker = picker.map(|picker| picker_snapshot(&picker.read(cx).view_model()));
        }
        snapshot.toasts = self
            .presenter
            .toasts()
            .visible()
            .iter()
            .map(|toast| toast.text().to_string())
            .collect();
        snapshot.status = self.status_message();
        snapshot
    }
}
