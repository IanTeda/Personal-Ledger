//! The Documents destination's Entities (ADR-0032). [`DocumentsStore`] reads the Documents and the
//! Document Types through `LedgerStore`; Settings › Documents edits the Types too.
//! [`DocumentsView`] owns the page's own state: mode, scope, sort, search and selection. The view reports what needs `Shell` (the OS
//! file hand-off) as a [`DocumentsEvent`].

use std::path::PathBuf;

use gpui::{App, Context, Entity, EventEmitter, ScrollHandle, Subscription};
use lib_ledger::Domain;

use super::DocumentsFocus;
use crate::documents::{
    Document, DocumentsMode, FilingUndo, LibraryScope, LibrarySort,
    types::{DocumentTypeRow, first_free_id},
};
use crate::ledger_store::{LedgerChanged, LedgerStore};

/// The Documents and the Types, handed to a mutation as one value so it can borrow them together.
pub struct DocumentsData<'a> {
    pub documents: &'a mut Vec<Document>,
    /// The Ledger's Document Types in the user's order (mock data until persistence lands).
    pub types: &'a mut Vec<DocumentTypeRow>,
    /// The id the next added Document Type takes; only counts up, so ids are never reused.
    pub types_next_id: &'a mut u32,
}

/// The shared Documents and Document Types, read through [`LedgerStore`]. The Documents page,
/// Settings › Documents, the rail badge and the Inventory removal all read them from here, so a
/// change is seen everywhere on the next render.
pub struct DocumentsStore {
    ledger: Entity<LedgerStore>,
    /// The id counter for Document Types. It's not a row, and it goes when ids become UUIDv7
    /// newtypes (ADR-0040), so it stays here rather than on the Ledger.
    types_next_id: u32,
    _ledger_changed: Subscription,
}

impl DocumentsStore {
    /// A store over `ledger`'s Documents and Document Types.
    pub fn new(ledger: Entity<LedgerStore>, cx: &mut Context<'_, Self>) -> Self {
        let types_next_id = first_free_id(ledger.read(cx).ledger().document_types());
        let _ledger_changed = cx.subscribe(&ledger, |_, _, event: &LedgerChanged, cx| {
            if event.what.contains(&Domain::Documents)
                || event.what.contains(&Domain::DocumentTypes)
            {
                cx.notify();
            }
        });
        Self {
            ledger,
            types_next_id,
            _ledger_changed,
        }
    }

    pub fn documents<'a>(&self, cx: &'a App) -> &'a [Document] {
        self.ledger.read(cx).ledger().documents()
    }

    pub fn types<'a>(&self, cx: &'a App) -> &'a [DocumentTypeRow] {
        self.ledger.read(cx).ledger().document_types()
    }

    /// Applies `change` to the data, then tells every subscriber it changed. Mutations go through
    /// here so no write can skip the notification.
    #[expect(
        deprecated,
        reason = "Documents and Document Types writes move onto Ledger commands in #645"
    )]
    pub fn mutate<R>(
        &mut self,
        cx: &mut App,
        change: impl FnOnce(&mut DocumentsData<'_>) -> R,
    ) -> R {
        let types_next_id = &mut self.types_next_id;
        self.ledger.update(cx, |ledger, cx| {
            ledger.write(cx, &[Domain::Documents, Domain::DocumentTypes], |ledger| {
                change(&mut DocumentsData {
                    documents: ledger.documents,
                    types: ledger.document_types,
                    types_next_id,
                })
            })
        })
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
