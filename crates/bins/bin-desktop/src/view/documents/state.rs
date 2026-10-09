//! The Documents destination's Entities (ADR-0032). [`DocumentsStore`] owns the Documents and the
//! Document Types, which Settings › Documents edits too; [`DocumentsView`] owns the page's own
//! state: mode, scope, sort, search and selection. The view reports what needs `Shell` (the OS
//! file hand-off) as a [`DocumentsEvent`].

use std::path::PathBuf;

use gpui::{Context, EventEmitter, ScrollHandle};

use super::DocumentsFocus;
use crate::documents::{
    Document, DocumentsMode, FilingUndo, LibraryScope, LibrarySort,
    types::{DocumentTypeRow, first_free_id},
};

/// Everything the store owns, handed to a mutation as one value so it can borrow the Documents and
/// the Types together.
pub struct DocumentsData {
    pub documents: Vec<Document>,
    /// The Ledger's Document Types in the user's order (mock data until persistence lands).
    pub types: Vec<DocumentTypeRow>,
    /// The id the next added Document Type takes; only counts up, so ids are never reused.
    pub types_next_id: u32,
}

/// The shared Documents and Document Types. The Documents page, Settings › Documents, the rail
/// badge and the Inventory removal all read them from here, so a change is seen everywhere on the
/// next render.
pub struct DocumentsStore {
    data: DocumentsData,
}

impl DocumentsStore {
    pub fn new(documents: Vec<Document>, types: Vec<DocumentTypeRow>) -> Self {
        let types_next_id = first_free_id(&types);
        Self {
            data: DocumentsData {
                documents,
                types,
                types_next_id,
            },
        }
    }

    pub fn documents(&self) -> &[Document] {
        &self.data.documents
    }

    pub fn types(&self) -> &[DocumentTypeRow] {
        &self.data.types
    }

    /// Applies `change` to the data, then tells every subscriber it changed. Mutations go through
    /// here so no write can skip the notification.
    pub fn mutate<R>(
        &mut self,
        cx: &mut Context<'_, Self>,
        change: impl FnOnce(&mut DocumentsData) -> R,
    ) -> R {
        let result = change(&mut self.data);
        cx.notify();
        result
    }
}

/// What the Documents page asks `Shell` to do. The page never touches the OS itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DocumentsEvent {
    /// `enter` / **Open** and `o` / **Show in folder**: hand the file to the OS. `Shell` runs it,
    /// since opening and revealing need an `App` a key handler doesn't have.
    OpenFile { path: PathBuf, reveal: bool },
}

impl EventEmitter<DocumentsEvent> for DocumentsView {}

/// The Documents page's own state, owned by its Entity.
pub struct DocumentsView {
    state: DocumentsState,
}

impl DocumentsView {
    pub fn new(state: DocumentsState) -> Self {
        Self { state }
    }

    pub fn state(&self) -> &DocumentsState {
        &self.state
    }

    /// Edits the page's state, then asks for a re-render.
    pub fn edit<R>(
        &mut self,
        cx: &mut Context<'_, Self>,
        change: impl FnOnce(&mut DocumentsState) -> R,
    ) -> R {
        let result = change(&mut self.state);
        cx.notify();
        result
    }

    pub fn request_open(&mut self, cx: &mut Context<'_, Self>, path: PathBuf, reveal: bool) {
        cx.emit(DocumentsEvent::OpenFile { path, reveal });
    }
}

pub struct DocumentsState {
    /// Inbox or Library. With the scope and sort, this persists across restarts.
    pub mode: DocumentsMode,
    pub scope: LibraryScope,
    pub sort: LibrarySort,
    /// The Library's search text: session-only, and kept across an `i` round-trip.
    pub query: String,
    /// Index rail or list, while the View zone has focus.
    pub focus: DocumentsFocus,
    /// The selected Library row, a position in the listed rows; clamped wherever it is read.
    pub selected: usize,
    pub scroll: ScrollHandle,
    /// The focused Inbox row, a position in the Inbox's rows; clamped wherever it is read. Kept
    /// apart from the Library's so an `i` round-trip returns to both.
    pub inbox_selected: usize,
    /// The last filing action as a unit, for `u`. Session-only; a new filing action replaces it.
    pub undo: Option<FilingUndo>,
}

impl Default for DocumentsState {
    fn default() -> Self {
        Self {
            mode: DocumentsMode::default(),
            scope: LibraryScope::default(),
            sort: LibrarySort::default(),
            query: String::new(),
            focus: DocumentsFocus::default(),
            selected: 0,
            scroll: ScrollHandle::new(),
            inbox_selected: 0,
            undo: None,
        }
    }
}
