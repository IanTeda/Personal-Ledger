//! What the integration tests read back of the Documents page, kept apart from the behaviour in
//! `shell/documents_ui.rs` so the snapshot shape and its accessor sit together.

use crate::documents::Document;
use crate::documents::form::DocumentsDialog;
use crate::documents::picker::Purpose;
use crate::shell::Shell;
use crate::view::documents::DocumentsFocus;

/// The Documents page's state: ids and plain values, so the private `documents` types need not
/// become public for them.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentsSnapshot {
    /// `library` or `inbox`.
    pub mode: &'static str,
    /// The Library scope's stable id, e.g. `all` or `type:receipt`.
    pub scope: String,
    /// `newest` or `expiring`.
    pub sort: &'static str,
    pub index_focused: bool,
    pub query: String,
    /// The file name of the Document the selection is on.
    pub selected_file: Option<String>,
    /// How many Links the selected Document has.
    pub selected_links: usize,
    /// Which dialog is open: `add`, `edit`, `import`, `facts`, `accept-all`, or
    /// `picker-link`, `picker-file`, `picker-follow`.
    pub dialog: Option<&'static str>,
    pub library_rows: usize,
    pub inbox_rows: usize,
    pub toasts: Vec<String>,
    pub status: Option<String>,
    /// The open picker's state, if one is open.
    pub picker: Option<PickerSnapshot>,
}

/// What a test reads of the open link picker.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickerSnapshot {
    /// The kind filter's label.
    pub kind: String,
    pub query: String,
    pub selected: usize,
    /// Each row's kind label, text and whether it is a ticked current Link; `link` is `None` on
    /// **File without link**.
    pub rows: Vec<(String, String, bool, bool)>,
    pub doc_type_chosen: bool,
}

impl Shell {
    #[doc(hidden)]
    pub fn documents_snapshot(&self) -> DocumentsSnapshot {
        DocumentsSnapshot {
            mode: self.documents_mode.id(),
            scope: self.documents_scope.id(),
            sort: self.documents_sort.id(),
            index_focused: self.documents_focus == DocumentsFocus::Index,
            query: self.documents_query.clone(),
            selected_file: self.documents_selected_document().map(Document::file_name),
            selected_links: self
                .documents_selected_document()
                .map_or(0, |document| document.links.len()),
            dialog: self.documents_dialog().map(|dialog| match dialog {
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
            }),
            library_rows: self.documents_library_rows().len(),
            inbox_rows: self.documents_inbox_rows().len(),
            toasts: self
                .chrome
                .toasts
                .visible()
                .iter()
                .map(|toast| toast.text().to_string())
                .collect(),
            status: self.chrome.status_message.clone(),
            picker: match self.documents_dialog() {
                Some(DocumentsDialog::Picker(state)) => Some(PickerSnapshot {
                    kind: state.kind.label(),
                    query: state.query.text().to_string(),
                    selected: state.selected,
                    rows: self
                        .documents_picker_rows(state)
                        .into_iter()
                        .map(|row| (row.kind, row.text, row.checked, row.link.is_some()))
                        .collect(),
                    doc_type_chosen: state.doc_type.is_some(),
                }),
                _ => None,
            },
        }
    }
}
