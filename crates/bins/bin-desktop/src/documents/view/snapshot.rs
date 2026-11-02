//! What the integration tests read back of the Documents page (ADR-0030), built from its
//! Presenter. The open dialog, the Toasts and the status message are `Root`'s, so its accessor
//! fills them in.

use gpui::App;
use lib_documents::presenter::DocumentsFocus;
use lib_documents::presenter::library::RowTail;

use super::page::DocumentsPage;
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
    /// The selected Library row's Key Date flag and whether it is red (Need Review), when it has
    /// one.
    pub selected_flag: Option<(String, bool)>,
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

impl DocumentsPage {
    /// The page's part of the snapshot; the dialog, Toasts and status are left empty for `Root`.
    #[doc(hidden)]
    pub fn snapshot(&self, _cx: &App) -> DocumentsSnapshot {
        let presenter = self.presenter();
        let (mode, scope, sort) = presenter.persisted();
        let selected = presenter.selected_document();
        DocumentsSnapshot {
            mode: mode.id(),
            scope: scope.id(),
            sort: sort.id(),
            index_focused: presenter.focus() == DocumentsFocus::Index,
            query: presenter.query().to_string(),
            selected_file: selected.as_ref().map(Document::file_name),
            selected_links: selected.as_ref().map_or(0, |document| document.links.len()),
            selected_chips: presenter.selected_chips(),
            selected_flag: presenter.view_model().library.and_then(|library| {
                match library.rows.get(library.selected)?.tail.clone() {
                    RowTail::Flag { text, red } => Some((text, red)),
                    RowTail::Type(_) => None,
                }
            }),
            dialog: None,
            library_rows: presenter.library_count(),
            inbox_rows: presenter.inbox_count(),
            toasts: Vec::new(),
            status: None,
            picker: None,
        }
    }
}
