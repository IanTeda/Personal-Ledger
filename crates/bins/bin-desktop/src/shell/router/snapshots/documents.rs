//! What the integration tests read back of the Documents page. The Documents Entity builds the
//! page's part; the open Dialog, the Toasts and the status message are `Root`'s, so they are
//! filled in here.

use crate::documents::view::dialog::DocumentsDialog;
#[doc(hidden)]
pub use crate::documents::view::snapshot::DocumentsSnapshot;
use crate::shell::router::Root;

impl Root {
    /// The Documents page for a test, as its Entity builds it, plus the open Documents Dialog.
    #[doc(hidden)]
    pub fn documents_snapshot(&self, cx: &gpui::App) -> DocumentsSnapshot {
        let view = self.open().views.documents.read(cx);
        let mut snapshot = view.snapshot(cx);
        let dialog = self
            .hosted_dialog()
            .and_then(|open| open.downcast::<DocumentsDialog>());
        if let Some(dialog) = dialog {
            let dialog = dialog.read(cx);
            snapshot.dialog = Some(dialog.kind());
            snapshot.picker = dialog.picker();
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
