//! What the integration tests read back of the Documents page (ADR-0030), built by the Documents
//! Entity from its own state (ADR-0037). The open dialog, the Toasts and the status message are
//! `Root`'s, so its accessor fills them in.

use gpui::App;

use super::{DocumentsFocus, state::DocumentsView};
use crate::documents::Document;

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
    /// The names of the selected Document's Links that resolve to a record, as its row's chips
    /// draw them: a Link whose record has gone draws none.
    pub selected_chips: Vec<String>,
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

impl DocumentsView {
    /// The page's part of the snapshot; the dialog, Toasts and status are left empty for `Root`.
    #[doc(hidden)]
    pub fn snapshot(&self, cx: &App) -> DocumentsSnapshot {
        let state = &self.state;
        let selected = self.selected_document(cx);
        DocumentsSnapshot {
            mode: state.mode.id(),
            scope: state.scope.id(),
            sort: state.sort.id(),
            index_focused: state.focus == DocumentsFocus::Index,
            query: state.query.clone(),
            selected_file: selected.map(Document::file_name),
            selected_links: selected.map_or(0, |document| document.links.len()),
            selected_chips: selected.map_or_else(Vec::new, |document| {
                crate::documents::model::row(document, &self.lookups(cx))
                    .chips
                    .into_iter()
                    .map(|chip| chip.label)
                    .collect()
            }),
            dialog: None,
            library_rows: self.library_rows(cx).len(),
            inbox_rows: self.inbox_rows(cx).len(),
            toasts: Vec::new(),
            status: None,
            picker: None,
        }
    }
}
