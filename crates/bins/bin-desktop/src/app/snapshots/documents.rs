//! What the integration tests read back of the Documents page. The Documents Entity builds the
//! page's part; the open dialog, the Toasts and the status message are `Shell`'s, so they are
//! filled in here.

use crate::app::Shell;
use crate::documents::form::DocumentsDialog;
use crate::documents::picker::Purpose;
#[doc(hidden)]
pub use crate::view::documents::snapshot::{DocumentsSnapshot, PickerSnapshot};

impl Shell {
    /// The Documents page for a test, as its Entity builds it, plus the open Documents dialog.
    #[doc(hidden)]
    pub fn documents_snapshot(&self, cx: &gpui::App) -> DocumentsSnapshot {
        let view = self.views.documents.read(cx);
        let mut snapshot = view.snapshot(cx);
        let dialog = self.dialog::<DocumentsDialog>();
        snapshot.dialog = dialog.map(|dialog| match dialog {
            DocumentsDialog::Add(_) => "add",
            DocumentsDialog::Edit(..) => "edit",
            DocumentsDialog::Import(..) => "import",
            DocumentsDialog::Facts(..) => "facts",
            DocumentsDialog::AcceptAll(_) => "accept-all",
            DocumentsDialog::Picker(state) => match state.purpose {
                Purpose::Link(_) => "picker-link",
                Purpose::File(_) => "picker-file",
                Purpose::Follow(_) => "picker-follow",
            },
        });
        snapshot.toasts = self
            .chrome
            .toasts
            .visible()
            .iter()
            .map(|toast| toast.text().to_string())
            .collect();
        snapshot.status = self.chrome.status_message.clone();
        snapshot.picker = match dialog {
            Some(DocumentsDialog::Picker(state)) => Some(PickerSnapshot {
                kind: state.kind.label(),
                query: state.query.text().to_string(),
                selected: state.selected,
                rows: view
                    .picker_rows(state, cx)
                    .into_iter()
                    .map(|row| (row.kind, row.text, row.checked, row.link.is_some()))
                    .collect(),
                doc_type_chosen: state.doc_type.is_some(),
            }),
            _ => None,
        };
        snapshot
    }
}
