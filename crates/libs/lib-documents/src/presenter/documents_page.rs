//! The Documents page (4a, 4b): the index rail, the Library's list and detail, and the Inbox's
//! list and detail. It holds what the page shows (Inbox or Library, the scope, the sort, the
//! Search, which zone has focus, each list's selection) and the session's filing state: the
//! Documents set aside with Skip, which is never written, and the last filing, which `u` reverses
//! as one unit. Both are lost on restart.
//!
//! The page reads the day from its Clock on every event and every view model, so a Key Date's band
//! and Need Review move on as the day does; a Financial Year scope that leaves the rail as the
//! year turns falls back to All documents. Writes go through [`crate::interactor`]'s Interactors,
//! and the Dialogs it opens through its Router, which hands what they did back as
//! [`DocumentsPageEvent::DialogClosed`].

use std::collections::HashSet;
use std::path::Path;

use lib_config::KeyBindingConfig;
use lib_core::{Clock, DateStyle, DocumentId, TransactionId};
use lib_list_cursor::Step;
use lib_toast::{Notice, ToastKind};
use lib_viper::display::PageDisplay;
use lib_viper::keymap::{EDIT, KeyBound, Keymap, MOVE_DOWN, MOVE_UP, SEARCH, SELECT};
use lib_viper::presenter::{PageMode, PagePresenter, Presenter, TextKey};
use lib_viper::view::{Hint, count};

use super::inbox::{self, InboxDetailView, InboxRowView};
use super::library::{self, DetailView, Lookups, RailRow, RowView, link_name, size_text};
use crate::interactor::{
    Accept, AcceptAllStrong, FileByHand, ImportDocuments, ToggleLink, UndoFiling,
};
use crate::keymap::{
    ACCEPT, ACCEPT_ALL, ADD, FOLLOW, IMPORT, LINK, SHOW_IN_FOLDER, SKIP, SORT, TOGGLE_MODE, UNDO,
    documents_page_keymap,
};
use crate::protocols::{
    DocumentsDialog, DocumentsOutcome, DocumentsRouter, SharedFiles, SharedReads, SharedRepository,
};
use crate::types::{self, DocumentTypeRow};
use crate::{
    Document, DocumentLink, DocumentType, DocumentsMode, FiledEntry, FilingUndo, ImportOutcome,
    LibraryScope, LibrarySort, LinkRecords, RailEntry, Source, focus_after_filing, inbox_count,
    inbox_rows, library_rows, msg, need_review_count, rail_entries, scope_count, strong_count,
    suggestion, total_bytes, year_facets,
};

/// Whether keyboard focus is on the index rail or the list, while the page has it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DocumentsFocus {
    Index,
    #[default]
    List,
}

/// What the page is told: a key, a click, or the Shell.
#[derive(Debug, Clone, PartialEq)]
pub enum DocumentsPageEvent {
    /// A movement in the zone with focus: the rail's scope, or the list's row.
    Move(Step),
    /// `enter`: from the index into the list; in the list, the file opened.
    Select,
    /// `l`: from the index into the list; in the list, the link picker (filing, in the Inbox).
    Link,
    /// From the index into the list.
    FocusList,
    /// From the list back to the index.
    FocusIndex,
    /// `L`: follows the selected Document's Link, or the Inbox row's Suggested Link.
    Follow,
    /// `i`: the Inbox or the Library, whichever isn't showing.
    ToggleMode,
    ShowInbox,
    ShowLibrary,
    /// `+ Add`, in the Library.
    Add,
    /// `Import…`.
    Import,
    /// The Library's sort, Newest or Expiring.
    Sort,
    /// `e`: Edit in the Library, Edit facts in the Inbox.
    Edit,
    /// `d`: the selected Document deleted after a confirm, in either mode. Its file and linked
    /// records stay.
    Delete,
    /// The selected file, opened in its application.
    Open,
    /// The selected file, shown in its folder.
    ShowInFolder,
    /// The focused Inbox row filed under its Suggested Link; filed by hand when it has none.
    Accept,
    /// Accept all strong matches, after a count-first confirm.
    AcceptAll,
    /// The focused Inbox row set aside for the session.
    Skip,
    /// The last filing reversed.
    Undo,
    /// `+ Link to…`, `Link elsewhere…` and `File…`: the link picker, whatever has focus.
    OpenPicker,
    /// A click on a rail row.
    ClickRail(RailEntry),
    /// A click on a list row.
    ClickRow(usize),
    /// A click on one of the sort's two options.
    ClickSort(LibrarySort),
    /// An Inbox row's own Accept (or File…).
    ClickAccept(usize),
    /// An Other candidate, accepted in place of the Suggested Link.
    AcceptCandidate(TransactionId),
    /// A LINKED TO row's ×: the Link removed at once.
    Unlink(DocumentLink),
    /// A LINKED TO row: the record it names.
    Reveal(DocumentLink),
    /// **Watched folder…**.
    WatchedFolder,
    /// `/` or a click on the Search box, in the Library.
    StartSearch,
    SearchType(char),
    SearchBackspace,
    /// `enter` in the Search: the text kept, back to the rows.
    KeepSearch,
    /// `esc`: the Search, then a kept Search's text, then the list back to the index.
    Back,
    /// Files dropped on the window, as typed paths: into the Inbox, the page staying where it is.
    Drop(Vec<String>),
    /// The page became the destination.
    Entered,
    /// The calendar month turned while the page was open.
    MonthTurned,
    /// The mode, scope and sort kept from the last run.
    Restore {
        mode: DocumentsMode,
        scope: LibraryScope,
        sort: LibrarySort,
    },
    /// A Dialog the page opened closed, with what its confirm did, or `None` for a cancel.
    DialogClosed(Option<DocumentsOutcome>),
}

impl DocumentsPageEvent {
    /// The half of the page this event acts in, for one that does nothing in the other (the
    /// palette shows it first); `None` for an event both take.
    pub fn mode(&self) -> Option<DocumentsMode> {
        match self {
            Self::Add | Self::Sort => Some(DocumentsMode::Library),
            Self::Accept | Self::AcceptAll | Self::Skip | Self::Undo => Some(DocumentsMode::Inbox),
            _ => None,
        }
    }
}

/// The page's static text: headings, buttons and placeholders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageLabels {
    pub import: String,
    pub add: String,
    pub search_placeholder: String,
    pub newest: String,
    pub expiring: String,
    pub not_linked: String,
    pub linked_to: String,
    pub link_add: String,
    pub open: String,
    pub show_in_folder: String,
    pub delete: String,
    pub watched_folder: String,
    pub file_column: String,
    pub link_column: String,
    pub inbox_empty: String,
    pub inbox_footnote: String,
    pub drop_target: String,
    pub suggested_link: String,
    pub other_candidates: String,
}

impl PageLabels {
    fn new() -> Self {
        Self {
            import: msg::documents_import_button(),
            add: msg::documents_add_button(),
            search_placeholder: msg::documents_search_placeholder(),
            newest: msg::documents_sort_newest(),
            expiring: msg::documents_sort_expiring(),
            not_linked: msg::documents_not_linked(),
            linked_to: msg::documents_linked_to(),
            link_add: msg::documents_link_add(),
            open: msg::documents_open(),
            show_in_folder: msg::documents_show_in_folder(),
            delete: msg::documents_delete(),
            watched_folder: msg::documents_watched_folder(),
            file_column: msg::documents_inbox_col_file(),
            link_column: msg::documents_inbox_col_link(),
            inbox_empty: msg::documents_inbox_empty(),
            inbox_footnote: msg::documents_inbox_footnote(),
            drop_target: msg::documents_drop_target(),
            suggested_link: msg::documents_suggested_link(),
            other_candidates: msg::documents_other_candidates(),
        }
    }
}

/// The Library's list and detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LibraryViewModel {
    pub query: String,
    pub searching: bool,
    pub sort: LibrarySort,
    /// "12 of 140 · 3 to review".
    pub count_line: String,
    pub rows: Vec<RowView>,
    /// The selected row, clamped to the rows.
    pub selected: usize,
    pub detail: Option<DetailView>,
    /// What an empty list says: nothing filed in the scope, or nothing matching the Search, which
    /// offers no `+ Add`.
    pub empty: String,
    pub empty_offers_add: bool,
}

/// The Inbox's list and detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxViewModel {
    /// Newest received first, the Skipped last.
    pub rows: Vec<InboxRowView>,
    /// The focused row, clamped to the rows.
    pub selected: usize,
    pub detail: Option<InboxDetailView>,
    /// How many have a Strong Suggested Link: the Accept all button's count, which is disabled
    /// at none.
    pub strong: usize,
    pub accept_all: String,
    /// The key strip under the detail.
    pub keys: Vec<Hint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentsPageViewModel {
    pub mode: DocumentsMode,
    pub focus: DocumentsFocus,
    pub title: String,
    pub subline: String,
    pub rail: Vec<RailRow>,
    /// The rail row the scope or mode selects.
    pub rail_selected: RailEntry,
    /// The stored size, under the rail and at the status line's right.
    pub stored: String,
    pub labels: PageLabels,
    /// `Some` while the Library shows.
    pub library: Option<LibraryViewModel>,
    /// `Some` while the Inbox shows.
    pub inbox: Option<InboxViewModel>,
    /// The status line's legend for the zone and mode.
    pub hints: Vec<Hint>,
    /// The `?` cheat sheet's Documents keys: the index's, the mode's, then the keys the legend
    /// has no room for.
    pub help: Vec<Hint>,
    /// Whether `esc` has something to back out of: the Search, a kept query, or the list.
    pub takes_back: bool,
    /// The Library's scope and sort, kept across a restart with the mode.
    pub scope: LibraryScope,
    pub sort: LibrarySort,
}

/// "3 more below": the Inbox rows under the last one in view, which only the View knows.
pub fn inbox_below(rows: usize) -> String {
    msg::documents_inbox_below(count(rows))
}

pub struct DocumentsPagePresenter {
    documents: SharedRepository,
    reads: SharedReads,
    files: SharedFiles,
    clock: Clock,
    /// Every key, which the palette finds each command in by name.
    keymap: Keymap<DocumentsPageEvent>,
    /// The keys while the Library or the Inbox shows: the other mode's verbs have no chord, so
    /// their keys fall through to the Shell. Add keeps `a` in the Inbox, doing nothing there, so
    /// it never falls through to the Shell's Insert.
    library_keys: Keymap<DocumentsPageEvent>,
    inbox_keys: Keymap<DocumentsPageEvent>,
    /// The keys while the Search is open: none, so every key types.
    search_keys: Keymap<DocumentsPageEvent>,
    /// The Ledger file's name, which the status line says the files are stored beside.
    ledger_file: String,
    date_style: Option<DateStyle>,
    mode: DocumentsMode,
    scope: LibraryScope,
    sort: LibrarySort,
    query: String,
    searching: bool,
    focus: DocumentsFocus,
    /// The Library's selected row and the Inbox's focused one, each a position in its rows,
    /// clamped wherever it is read, and kept apart so an `i` round-trip returns to both.
    selected: usize,
    inbox_selected: usize,
    skipped: HashSet<DocumentId>,
    undo: Option<FilingUndo>,
}

impl DocumentsPagePresenter {
    /// The Library on All documents, Newest first, focus on the list, its keys under `config`.
    pub fn new(
        documents: SharedRepository,
        reads: SharedReads,
        files: SharedFiles,
        clock: Clock,
        config: &KeyBindingConfig,
        ledger_file: String,
    ) -> Self {
        let (keymap, library_keys, inbox_keys, search_keys) = keymaps(config);
        Self {
            documents,
            reads,
            files,
            clock,
            keymap,
            library_keys,
            inbox_keys,
            search_keys,
            ledger_file,
            date_style: None,
            mode: DocumentsMode::default(),
            scope: LibraryScope::default(),
            sort: LibrarySort::default(),
            query: String::new(),
            searching: false,
            focus: DocumentsFocus::default(),
            selected: 0,
            inbox_selected: 0,
            skipped: HashSet::new(),
            undo: None,
        }
    }

    pub fn mode(&self) -> DocumentsMode {
        self.mode
    }

    /// The Library's scope, All documents once its Document Type has gone.
    pub fn scope(&self) -> LibraryScope {
        match self.scope {
            LibraryScope::Type(DocumentType(id))
                if types::get(&self.document_types(), id).is_none() =>
            {
                LibraryScope::All
            }
            scope => scope,
        }
    }

    pub fn sort(&self) -> LibrarySort {
        self.sort
    }

    pub fn focus(&self) -> DocumentsFocus {
        self.focus
    }

    pub fn query(&self) -> &str {
        &self.query
    }

    /// The day the page is worked out on.
    pub fn today(&self) -> chrono::NaiveDate {
        self.clock.today()
    }

    /// Whether the Search is open: the Client's Search mode.
    pub fn searching(&self) -> bool {
        self.searching
    }

    /// The mode, scope and sort the Client keeps across a restart.
    pub fn persisted(&self) -> (DocumentsMode, LibraryScope, LibrarySort) {
        (self.mode, self.scope(), self.sort)
    }

    /// The Document the selection is on: the Library's row or the Inbox's focused row.
    pub fn selected_document(&self) -> Option<Document> {
        let documents = self.all_documents();
        let types = self.document_types();
        match self.mode {
            DocumentsMode::Library => {
                let rows = self.library_rows(&types, &documents);
                let at = self.selected.min(rows.len().saturating_sub(1));
                rows.get(at).map(|document| (*document).clone())
            }
            DocumentsMode::Inbox => {
                let rows = inbox_rows(&documents, &self.skipped);
                let at = self.inbox_selected.min(rows.len().saturating_sub(1));
                rows.get(at).map(|document| (*document).clone())
            }
        }
    }

    /// How many rows the list showing has: the Library's under its scope and Search, or the
    /// Inbox's.
    pub fn row_count(&self) -> usize {
        match self.mode {
            DocumentsMode::Library => self.library_count(),
            DocumentsMode::Inbox => self.inbox_count(),
        }
    }

    /// How many rows the Library has under its scope and Search, whichever list shows.
    pub fn library_count(&self) -> usize {
        self.library_rows(&self.document_types(), &self.all_documents())
            .len()
    }

    /// How many rows the Inbox has, whichever list shows.
    pub fn inbox_count(&self) -> usize {
        self.all_documents()
            .iter()
            .filter(|document| document.is_unfiled())
            .count()
    }

    /// The selected row of the list showing, clamped to its rows.
    pub fn selected(&self) -> usize {
        let at = match self.mode {
            DocumentsMode::Library => self.selected,
            DocumentsMode::Inbox => self.inbox_selected,
        };
        at.min(self.row_count().saturating_sub(1))
    }

    /// The chips the selected Document's row draws: a Link whose record has gone draws none.
    pub fn selected_chips(&self) -> Vec<String> {
        let Some(document) = self.selected_document() else {
            return Vec::new();
        };
        let types = self.document_types();
        let records = self.records();
        library::row(&document, &self.lookups(&types, &records))
            .chips
            .into_iter()
            .map(|chip| chip.label)
            .collect()
    }

    fn all_documents(&self) -> Vec<Document> {
        self.documents.borrow().documents()
    }

    fn document_types(&self) -> Vec<DocumentTypeRow> {
        self.documents.borrow().document_types()
    }

    fn records(&self) -> LinkRecords {
        LinkRecords::read(&*self.reads)
    }

    fn lookups<'a>(&self, types: &'a [DocumentTypeRow], records: &'a LinkRecords) -> Lookups<'a> {
        Lookups {
            types,
            records,
            today: self.clock.today(),
            date_style: self.date_style,
        }
    }

    fn library_rows<'a>(
        &self,
        types: &[DocumentTypeRow],
        documents: &'a [Document],
    ) -> Vec<&'a Document> {
        library_rows(
            types,
            documents,
            self.scope(),
            &self.query,
            self.sort,
            self.clock.today(),
        )
    }

    fn inbox_position(&self, id: DocumentId) -> Option<usize> {
        let documents = self.all_documents();
        inbox_rows(&documents, &self.skipped)
            .iter()
            .position(|document| document.id == id)
    }

    /// A scope on a Document Type that has gone, or on a Financial Year the rail no longer
    /// shows since the year turned, falls back to All documents.
    fn catch_up(&mut self) {
        let stale_year = matches!(
            self.scope,
            LibraryScope::Year(facet) if !year_facets(self.clock.today()).contains(&facet)
        );
        if stale_year || self.scope() != self.scope {
            self.scope = LibraryScope::All;
            self.selected = 0;
        }
    }

    fn set_mode(&mut self, mode: DocumentsMode) {
        self.mode = mode;
    }

    fn select_entry(&mut self, entry: RailEntry) {
        match entry {
            RailEntry::Inbox => self.set_mode(DocumentsMode::Inbox),
            RailEntry::Scope(scope) => {
                self.set_mode(DocumentsMode::Library);
                if self.scope != scope {
                    self.scope = scope;
                    self.selected = 0;
                }
            }
        }
    }

    fn move_rail(&mut self, step: Step) {
        let entries = rail_entries(&self.document_types(), self.clock.today());
        let current = match self.mode {
            DocumentsMode::Inbox => RailEntry::Inbox,
            DocumentsMode::Library => RailEntry::Scope(self.scope()),
        };
        let at = entries
            .iter()
            .position(|entry| *entry == current)
            .unwrap_or(0);
        let next = lib_list_cursor::step(at, entries.len(), step);
        if let Some(entry) = entries.get(next) {
            self.select_entry(*entry);
        }
    }

    fn move_row(&mut self, step: Step) {
        let len = self.row_count();
        if len == 0 {
            return;
        }
        let next = lib_list_cursor::step(self.selected(), len, step);
        match self.mode {
            DocumentsMode::Library => self.selected = next,
            DocumentsMode::Inbox => self.inbox_selected = next,
        }
    }

    /// `esc`, innermost first: the Search (its text cleared), then a kept Search's text, then
    /// the list back to the index.
    fn back(&mut self) {
        if self.searching || !self.query.is_empty() {
            self.searching = false;
            self.query.clear();
            self.selected = 0;
        } else if self.focus == DocumentsFocus::List {
            self.focus = DocumentsFocus::Index;
        }
    }

    /// Puts the Library on `id`, on All documents when its scope doesn't hold it and with the
    /// Search cleared, so a Document just added or edited is on screen.
    fn show_document(&mut self, id: DocumentId) {
        let documents = self.all_documents();
        let types = self.document_types();
        let in_scope = crate::get(&documents, id)
            .is_some_and(|document| self.scope().contains(document, &types));
        if !in_scope {
            self.scope = LibraryScope::All;
        }
        self.set_mode(DocumentsMode::Library);
        self.focus = DocumentsFocus::List;
        self.query.clear();
        self.searching = false;
        self.selected = self
            .library_rows(&types, &documents)
            .iter()
            .position(|document| document.id == id)
            .unwrap_or(0);
    }

    fn no_document(router: &mut dyn DocumentsRouter) {
        router.notify(Notice::Status(msg::documents_status_no_document()));
    }

    /// Hands the selected file to the platform when it exists, and says so when it doesn't.
    fn open_selected(&mut self, in_folder: bool, router: &mut dyn DocumentsRouter) {
        let Some(document) = self.selected_document() else {
            Self::no_document(router);
            return;
        };
        let path = self.files.resolve(&document.path.to_string_lossy());
        if !self.files.exists(&path) {
            router.notify(Notice::Toast(
                ToastKind::Warning,
                msg::documents_status_missing(&document.file_name(), &path.to_string_lossy()),
            ));
            return;
        }
        router.open_file(&path, in_folder);
        let name = file_name(&path);
        router.notify(Notice::Status(if in_folder {
            msg::documents_status_shown(&name)
        } else {
            msg::documents_status_opened(&name)
        }));
    }

    /// `e`: Edit in the Library, the Extracted Facts in the Inbox.
    fn edit(&mut self, router: &mut dyn DocumentsRouter) {
        let Some(document) = self.selected_document() else {
            Self::no_document(router);
            return;
        };
        router.open_dialog(match self.mode {
            DocumentsMode::Library => DocumentsDialog::Edit(document.id),
            DocumentsMode::Inbox => DocumentsDialog::Facts(document.id),
        });
    }

    /// `d` / **Delete**: asks to delete the selected Document, whichever mode shows.
    fn delete_selected(&mut self, router: &mut dyn DocumentsRouter) {
        let Some(document) = self.selected_document() else {
            Self::no_document(router);
            return;
        };
        router.open_dialog(DocumentsDialog::Delete(document.id));
    }

    /// Document `id` has been deleted: it leaves the pending Undo (which is cleared when that
    /// leaves it empty) and the skipped set, and the selection, which stays at its position in the
    /// list, now rests on the next row, or the previous one when `id` was the last. The Library's
    /// scope and Search are kept, even when they are left empty.
    fn deleted(&mut self, id: DocumentId) {
        self.undo = self.undo.take().and_then(|undo| undo.without(id));
        self.skipped.remove(&id);
        self.selected = self.selected.min(self.library_count().saturating_sub(1));
        self.inbox_selected = self
            .inbox_selected
            .min(self.inbox_count().saturating_sub(1));
    }

    /// The one link picker: toggling a Link in the Library, filing the focused Document in the
    /// Inbox.
    fn open_picker(&mut self, router: &mut dyn DocumentsRouter) {
        let Some(document) = self.selected_document() else {
            Self::no_document(router);
            return;
        };
        router.open_dialog(match self.mode {
            DocumentsMode::Library => DocumentsDialog::Link(document.id),
            DocumentsMode::Inbox => DocumentsDialog::File(document.id),
        });
    }

    /// `L`: in the Library one Link reveals its record, several open the picker over them, and none
    /// is a quiet no-op. In the Inbox it reveals the row's Suggested Link, so it can be checked
    /// before `y`.
    fn follow(&mut self, router: &mut dyn DocumentsRouter) {
        let Some(document) = self.selected_document() else {
            return;
        };
        match self.mode {
            DocumentsMode::Library => match document.links.as_slice() {
                [] => {}
                [only] => router.reveal(*only),
                _ => {
                    router.open_dialog(DocumentsDialog::Follow(document.id));
                }
            },
            DocumentsMode::Inbox => {
                if let Some(transaction_id) = self.suggested(&document) {
                    router.reveal(DocumentLink::Transaction(transaction_id));
                }
            }
        }
    }

    /// The Transaction `document`'s Suggested Link names, if it has one.
    fn suggested(&self, document: &Document) -> Option<TransactionId> {
        let records = self.records();
        suggestion(
            document,
            &self.all_documents(),
            &records.transactions,
            &records.payees,
        )
        .best()
        .map(|best| best.transaction_id)
    }

    /// Removes `link` from the selected Document at once, with a Toast and no confirm.
    fn unlink(&mut self, link: DocumentLink, router: &mut dyn DocumentsRouter) {
        let Some(document) = self.selected_document() else {
            return;
        };
        let types = self.document_types();
        let records = self.records();
        let name = link_name(link, &self.lookups(&types, &records)).unwrap_or_default();
        let toggled = ToggleLink {
            id: document.id,
            link,
        }
        .run(&mut *self.documents.borrow_mut());
        if toggled == Ok(false) {
            router.notify(Notice::Toast(
                ToastKind::Info,
                msg::documents_toast_unlinked(&name),
            ));
        }
    }

    /// `y` and **Accept & next**: files the focused Document under its Suggested Link, or through
    /// the picker when it has none.
    fn accept_focused(&mut self, router: &mut dyn DocumentsRouter) {
        let Some(document) = self.selected_document() else {
            return;
        };
        match self.suggested(&document) {
            Some(transaction_id) => self.accept(document.id, transaction_id, router),
            None => {
                router.open_dialog(DocumentsDialog::File(document.id));
            }
        }
    }

    fn accept(
        &mut self,
        id: DocumentId,
        transaction_id: TransactionId,
        router: &mut dyn DocumentsRouter,
    ) {
        let before = self.before_filing(id);
        let filed =
            Accept { id, transaction_id }.run(&*self.reads, &mut *self.documents.borrow_mut());
        if let Ok(entry) = filed {
            self.after_filing(id, before, entry, router);
        }
    }

    fn file_by_hand(
        &mut self,
        id: DocumentId,
        doc_type: DocumentType,
        link: Option<DocumentLink>,
        router: &mut dyn DocumentsRouter,
    ) {
        let before = self.before_filing(id);
        let filed = FileByHand { id, doc_type, link }.run(&mut *self.documents.borrow_mut());
        if let Ok(entry) = filed {
            self.after_filing(id, before, entry, router);
        }
    }

    /// Where `id` sits in the Inbox, how many rows it has, and what the Document is called, read
    /// before it leaves the Inbox.
    fn before_filing(&self, id: DocumentId) -> (Option<usize>, usize, String) {
        let documents = self.all_documents();
        let rows = inbox_rows(&documents, &self.skipped);
        (
            rows.iter().position(|document| document.id == id),
            rows.len(),
            crate::get(&documents, id)
                .map(|document| document.title.clone())
                .unwrap_or_default(),
        )
    }

    /// What every filing does: remembers it as the one undoable action, moves focus to the next
    /// row and raises the Toast.
    fn after_filing(
        &mut self,
        id: DocumentId,
        (at, len, title): (Option<usize>, usize, String),
        entry: FiledEntry,
        router: &mut dyn DocumentsRouter,
    ) {
        self.undo = Some(FilingUndo {
            entries: vec![entry],
            focus: Some(id),
        });
        if let Some(at) = at {
            self.inbox_selected = focus_after_filing(at, len).unwrap_or(0);
        }
        router.notify(Notice::Toast(
            ToastKind::Success,
            msg::documents_toast_filed(&title),
        ));
    }

    /// `x` / **Skip**: sets the focused Document aside to the bottom of the Inbox for the session.
    /// Focus stays at the same position, which is now the next row. Writes nothing.
    fn skip_focused(&mut self) {
        if let Some(document) = self.selected_document() {
            let _ = crate::skip(&self.all_documents(), &mut self.skipped, document.id);
        }
    }

    /// `u`: reverses the last filing as a unit and puts focus back on its Document, in the Inbox.
    fn undo_last(&mut self, router: &mut dyn DocumentsRouter) {
        let Some(record) = self.undo.take() else {
            router.notify(Notice::Status(msg::documents_status_nothing_to_undo()));
            return;
        };
        let undone = record.entries.len();
        let focus = UndoFiling(record).run(&mut *self.documents.borrow_mut());
        self.set_mode(DocumentsMode::Inbox);
        if let Some(at) = focus.and_then(|id| self.inbox_position(id)) {
            self.inbox_selected = at;
        }
        router.notify(Notice::Toast(
            ToastKind::Info,
            msg::documents_toast_undone(count(undone)),
        ));
    }

    /// Accept all asks first, with the count; with none Strong it says so instead.
    fn ask_accept_all(&mut self, router: &mut dyn DocumentsRouter) {
        let records = self.records();
        let strong = strong_count(
            &self.all_documents(),
            &records.transactions,
            &records.payees,
        );
        if strong == 0 {
            router.notify(Notice::Status(msg::documents_status_no_strong()));
            return;
        }
        router.open_dialog(DocumentsDialog::AcceptAll(strong));
    }

    /// Files every Strong match as one batch, keeping the focused row in view and the skipped
    /// last; nothing happens when none is Strong any more.
    fn accept_all(&mut self, router: &mut dyn DocumentsRouter) {
        let batch = AcceptAllStrong {
            focus: self.selected_document().map(|document| document.id),
            skipped: self.skipped.clone(),
        }
        .run(&*self.reads, &mut *self.documents.borrow_mut());
        let Some(batch) = batch else {
            return;
        };
        let filed = batch.entries.len();
        self.undo = Some(batch);
        self.inbox_selected = 0;
        router.notify(Notice::Toast(
            ToastKind::Success,
            msg::documents_toast_filed_many(&filed.to_string()),
        ));
    }

    /// Files dropped on the window land in the Inbox as `Import…` does, but the page stays where
    /// it is: the Toast and the rail's badge say what arrived.
    fn drop_files(&mut self, typed: Vec<String>, router: &mut dyn DocumentsRouter) {
        let outcomes = ImportDocuments {
            typed,
            today: self.clock.today(),
            source: Source::Dropped,
        }
        .run(&*self.files, &mut *self.documents.borrow_mut());
        let imported = outcomes
            .iter()
            .filter(|outcome| matches!(outcome, ImportOutcome::Imported(_)))
            .count();
        let refused = outcomes.len() - imported;
        let notice = match (imported, refused) {
            (0, 0) => return,
            (0, _) => Notice::Toast(
                ToastKind::Error,
                msg::documents_toast_import_partial("0", &refused.to_string()),
            ),
            (_, 0) => Notice::Toast(
                ToastKind::Success,
                msg::documents_toast_imported(count(imported)),
            ),
            (_, _) => Notice::Toast(
                ToastKind::Success,
                msg::documents_toast_import_partial(&imported.to_string(), &refused.to_string()),
            ),
        };
        router.notify(notice);
    }

    fn dialog_closed(&mut self, outcome: DocumentsOutcome, router: &mut dyn DocumentsRouter) {
        match outcome {
            DocumentsOutcome::Shown(id) => self.show_document(id),
            DocumentsOutcome::Imported => self.set_mode(DocumentsMode::Inbox),
            DocumentsOutcome::FactsSaved(id) => {
                // Editing clears a skip, so the Document may have moved up the Inbox: follow it.
                self.skipped.remove(&id);
                if let Some(at) = self.inbox_position(id) {
                    self.inbox_selected = at;
                }
            }
            DocumentsOutcome::AcceptAll => self.accept_all(router),
            DocumentsOutcome::FileByHand { id, doc_type, link } => {
                self.file_by_hand(id, doc_type, link, router);
            }
            DocumentsOutcome::Follow(link) => router.reveal(link),
            DocumentsOutcome::Deleted(id) => self.deleted(id),
        }
    }

    /// What the page's Dialogs read and write through, for the Client to build one on.
    pub fn dialog_ports(&self) -> super::DialogPorts {
        super::DialogPorts {
            documents: self.documents.clone(),
            reads: self.reads.clone(),
            files: self.files.clone(),
            today: self.clock.today(),
            date_style: self.date_style,
        }
    }

    /// The Documents set aside with Skip, for Accept all, which files them last.
    pub fn skipped(&self) -> &HashSet<DocumentId> {
        &self.skipped
    }

    fn rows_hint(&self) -> Hint {
        Hint::new(
            self.keymap.hint_chords(&[MOVE_DOWN, MOVE_UP]),
            msg::documents_hint_row(),
        )
    }

    /// The Inbox's legend, also its key strip under the detail.
    fn inbox_hints(&self) -> Vec<Hint> {
        vec![
            self.rows_hint(),
            Hint::new(self.keymap.hint_chord(ACCEPT), msg::documents_hint_accept()),
            Hint::new(self.keymap.hint_chord(EDIT), msg::documents_hint_edit()),
            Hint::new(self.keymap.hint_chord(SKIP), msg::documents_hint_skip()),
            Hint::new(
                self.keymap.hint_chord(LINK),
                msg::documents_hint_link_elsewhere(),
            ),
            Hint::new(self.keymap.hint_chord(FOLLOW), msg::documents_hint_follow()),
        ]
    }

    /// The status line's legend for the zone with focus and the mode showing.
    fn hints(&self) -> Vec<Hint> {
        self.legend(self.focus)
    }

    /// The legend for `focus` in the mode showing.
    fn legend(&self, focus: DocumentsFocus) -> Vec<Hint> {
        match (focus, self.mode) {
            (DocumentsFocus::Index, _) => vec![
                Hint::new(
                    self.keymap.hint_chords(&[MOVE_DOWN, MOVE_UP]),
                    msg::documents_hint_scope(),
                ),
                Hint::new(
                    self.keymap.hint_chords(&[LINK, SELECT]),
                    msg::documents_hint_list(),
                ),
                Hint::new(
                    self.keymap.hint_chord(TOGGLE_MODE),
                    msg::documents_hint_inbox(),
                ),
            ],
            (DocumentsFocus::List, DocumentsMode::Library) => vec![
                self.rows_hint(),
                Hint::new(self.keymap.hint_chord(SELECT), msg::documents_hint_open()),
                // The Shell's key, which sends the page's `search` by name.
                Hint::new(
                    self.keymap.hint_chord(SEARCH),
                    lib_locale::msg::hint_search(),
                ),
                Hint::new(self.keymap.hint_chord(LINK), msg::documents_hint_link()),
                Hint::new(self.keymap.hint_chord(FOLLOW), msg::documents_hint_follow()),
                Hint::new(
                    self.keymap.hint_chord(SHOW_IN_FOLDER),
                    msg::documents_hint_show_in_folder(),
                ),
                Hint::new(
                    self.keymap.hint_chord(TOGGLE_MODE),
                    msg::documents_hint_inbox(),
                ),
            ],
            (DocumentsFocus::List, DocumentsMode::Inbox) => self.inbox_hints(),
        }
    }

    /// The `?` cheat sheet's Documents keys: the index's, the mode's, then the keys the legend has
    /// no room for.
    fn help(&self) -> Vec<Hint> {
        let index = self.legend(DocumentsFocus::Index);
        let list = self.legend(DocumentsFocus::List);
        let more = match self.mode {
            DocumentsMode::Library => vec![
                Hint::new(self.keymap.hint_chord(SORT), msg::documents_hint_sort()),
                Hint::new(self.keymap.hint_chord(EDIT), msg::documents_hint_edit()),
                Hint::new(self.keymap.hint_chord(ADD), msg::documents_hint_add()),
            ],
            DocumentsMode::Inbox => vec![
                Hint::new(self.keymap.hint_chord(UNDO), msg::documents_hint_undo()),
                Hint::new(
                    self.keymap.hint_chord(ACCEPT_ALL),
                    msg::documents_hint_accept_all(),
                ),
            ],
        };
        index
            .into_iter()
            .chain(list)
            .chain(more)
            .chain([Hint::new(
                self.keymap.hint_chord(IMPORT),
                msg::documents_hint_import(),
            )])
            .collect()
    }
}

impl Presenter for DocumentsPagePresenter {
    type Event = DocumentsPageEvent;
    type ViewModel = DocumentsPageViewModel;
    type Router<'r> = dyn DocumentsRouter + 'r;

    fn handle(&mut self, event: DocumentsPageEvent, router: &mut Self::Router<'_>) {
        self.catch_up();
        match event {
            DocumentsPageEvent::Move(step) => match self.focus {
                DocumentsFocus::Index => self.move_rail(step),
                DocumentsFocus::List => self.move_row(step),
            },
            DocumentsPageEvent::Select => match self.focus {
                DocumentsFocus::Index => self.focus = DocumentsFocus::List,
                DocumentsFocus::List => self.open_selected(false, router),
            },
            DocumentsPageEvent::Link => match self.focus {
                DocumentsFocus::Index => self.focus = DocumentsFocus::List,
                DocumentsFocus::List => self.open_picker(router),
            },
            DocumentsPageEvent::FocusList => self.focus = DocumentsFocus::List,
            DocumentsPageEvent::FocusIndex => self.focus = DocumentsFocus::Index,
            DocumentsPageEvent::Follow => self.follow(router),
            DocumentsPageEvent::ToggleMode => self.set_mode(match self.mode {
                DocumentsMode::Library => DocumentsMode::Inbox,
                DocumentsMode::Inbox => DocumentsMode::Library,
            }),
            DocumentsPageEvent::ShowInbox => self.set_mode(DocumentsMode::Inbox),
            DocumentsPageEvent::ShowLibrary => self.set_mode(DocumentsMode::Library),
            DocumentsPageEvent::Add => {
                if self.mode == DocumentsMode::Library {
                    router.open_dialog(DocumentsDialog::Add(self.scope()));
                }
            }
            DocumentsPageEvent::Import => {
                router.open_dialog(DocumentsDialog::Import);
            }
            DocumentsPageEvent::Sort => {
                if self.mode == DocumentsMode::Library {
                    self.sort = self.sort.toggle();
                    self.selected = 0;
                }
            }
            DocumentsPageEvent::ClickSort(sort) => {
                if self.sort != sort {
                    self.sort = sort;
                    self.selected = 0;
                }
            }
            DocumentsPageEvent::Edit => self.edit(router),
            DocumentsPageEvent::Delete => self.delete_selected(router),
            DocumentsPageEvent::Open => self.open_selected(false, router),
            DocumentsPageEvent::ShowInFolder => self.open_selected(true, router),
            DocumentsPageEvent::Accept => self.accept_focused(router),
            DocumentsPageEvent::AcceptAll => self.ask_accept_all(router),
            DocumentsPageEvent::Skip => self.skip_focused(),
            DocumentsPageEvent::Undo => self.undo_last(router),
            DocumentsPageEvent::OpenPicker => self.open_picker(router),
            DocumentsPageEvent::ClickRail(entry) => {
                self.select_entry(entry);
                self.focus = DocumentsFocus::Index;
            }
            DocumentsPageEvent::ClickRow(index) => {
                match self.mode {
                    DocumentsMode::Library => self.selected = index,
                    DocumentsMode::Inbox => self.inbox_selected = index,
                }
                self.focus = DocumentsFocus::List;
            }
            DocumentsPageEvent::ClickAccept(index) => {
                self.inbox_selected = index;
                self.focus = DocumentsFocus::List;
                self.accept_focused(router);
            }
            DocumentsPageEvent::AcceptCandidate(transaction_id) => {
                if let Some(document) = self.selected_document() {
                    self.accept(document.id, transaction_id, router);
                }
            }
            DocumentsPageEvent::Unlink(link) => self.unlink(link, router),
            DocumentsPageEvent::Reveal(link) => router.reveal(link),
            DocumentsPageEvent::WatchedFolder => router.notify(Notice::Toast(
                ToastKind::Info,
                msg::documents_watched_folder_toast(),
            )),
            DocumentsPageEvent::StartSearch => {
                if self.mode == DocumentsMode::Library {
                    self.focus = DocumentsFocus::List;
                    self.searching = true;
                }
            }
            DocumentsPageEvent::SearchType(ch) => {
                if self.searching {
                    self.query.push(ch);
                    self.selected = 0;
                }
            }
            DocumentsPageEvent::SearchBackspace => {
                if self.searching {
                    self.query.pop();
                    self.selected = 0;
                }
            }
            DocumentsPageEvent::KeepSearch => self.searching = false,
            DocumentsPageEvent::Back => self.back(),
            DocumentsPageEvent::Drop(paths) => self.drop_files(paths, router),
            DocumentsPageEvent::Entered => {
                self.focus = DocumentsFocus::List;
                self.searching = false;
            }
            DocumentsPageEvent::MonthTurned => {}
            DocumentsPageEvent::Restore { mode, scope, sort } => {
                self.mode = mode;
                self.scope = scope;
                self.sort = sort;
            }
            DocumentsPageEvent::DialogClosed(Some(outcome)) => self.dialog_closed(outcome, router),
            DocumentsPageEvent::DialogClosed(None) => {}
        }
    }

    fn view_model(&self) -> DocumentsPageViewModel {
        let documents = self.all_documents();
        let types = self.document_types();
        let records = self.records();
        let lookups = self.lookups(&types, &records);
        let today = self.clock.today();
        let filed = scope_count(&types, &documents, LibraryScope::All);
        let library = (self.mode == DocumentsMode::Library).then(|| {
            let rows = self.library_rows(&types, &documents);
            let selected = self.selected.min(rows.len().saturating_sub(1));
            let searched = !self.query.trim().is_empty();
            LibraryViewModel {
                query: self.query.clone(),
                searching: self.searching,
                sort: self.sort,
                count_line: msg::documents_count_line(
                    &rows.len().to_string(),
                    &filed.to_string(),
                    &need_review_count(&types, &rows, today).to_string(),
                ),
                detail: rows
                    .get(selected)
                    .map(|document| library::detail(document, &lookups)),
                rows: rows
                    .iter()
                    .map(|document| library::row(document, &lookups))
                    .collect(),
                selected,
                empty: if searched {
                    msg::documents_empty_search()
                } else {
                    msg::documents_empty_scope()
                },
                empty_offers_add: !searched,
            }
        });
        let inbox = (self.mode == DocumentsMode::Inbox).then(|| {
            let rows = inbox_rows(&documents, &self.skipped);
            let selected = self.inbox_selected.min(rows.len().saturating_sub(1));
            let strong = strong_count(&documents, &records.transactions, &records.payees);
            InboxViewModel {
                detail: rows
                    .get(selected)
                    .map(|document| inbox::detail(document, &documents, &lookups)),
                rows: rows
                    .iter()
                    .map(|document| {
                        let skipped = self.skipped.contains(&document.id);
                        inbox::row(document, &documents, &lookups, skipped)
                    })
                    .collect(),
                selected,
                strong,
                accept_all: msg::documents_accept_all(&strong.to_string()),
                keys: self.inbox_hints(),
            }
        });
        DocumentsPageViewModel {
            mode: self.mode,
            focus: self.focus,
            title: match self.mode {
                DocumentsMode::Library => msg::documents_library_title(),
                DocumentsMode::Inbox => msg::documents_inbox_title(),
            },
            subline: match self.mode {
                DocumentsMode::Library => msg::documents_subline(count(filed)),
                DocumentsMode::Inbox => {
                    msg::documents_inbox_subline(count(inbox_count(&documents)))
                }
            },
            rail: library::rail_rows(&types, &documents, today),
            rail_selected: match self.mode {
                DocumentsMode::Inbox => RailEntry::Inbox,
                DocumentsMode::Library => RailEntry::Scope(self.scope()),
            },
            stored: msg::documents_status_stored(
                &size_text(total_bytes(&documents)),
                &self.ledger_file,
            ),
            labels: PageLabels::new(),
            library,
            inbox,
            hints: self.hints(),
            help: self.help(),
            takes_back: self.searching
                || !self.query.is_empty()
                || self.focus == DocumentsFocus::List,
            scope: self.scope(),
            sort: self.sort,
        }
    }

    /// Whether `event` does anything where focus is; the Client offers a key it doesn't take on
    /// to the Shell. A key of the other mode has no chord there ([`PagePresenter::keymap`]), so an
    /// event of the other mode comes only from the palette, after [`PagePresenter::shows`] has
    /// shown that mode.
    fn takes(&self, event: &DocumentsPageEvent) -> bool {
        let list = self.focus == DocumentsFocus::List;
        let inbox = self.mode == DocumentsMode::Inbox;
        match event {
            DocumentsPageEvent::Sort => !inbox,
            DocumentsPageEvent::Accept | DocumentsPageEvent::Skip => inbox && list,
            DocumentsPageEvent::AcceptAll | DocumentsPageEvent::Undo => inbox,
            DocumentsPageEvent::FocusList => !list,
            DocumentsPageEvent::FocusIndex
            | DocumentsPageEvent::Follow
            | DocumentsPageEvent::ShowInFolder => list,
            _ => true,
        }
    }
}

impl PagePresenter for DocumentsPagePresenter {
    type Outcome = DocumentsOutcome;

    fn dialog_closed(outcome: Option<DocumentsOutcome>) -> Option<DocumentsPageEvent> {
        Some(DocumentsPageEvent::DialogClosed(outcome))
    }

    /// No keys while the Search is open, so every key types; else the mode showing's.
    fn keymap(&self) -> &Keymap<DocumentsPageEvent> {
        if self.searching {
            &self.search_keys
        } else {
            match self.mode {
                DocumentsMode::Library => &self.library_keys,
                DocumentsMode::Inbox => &self.inbox_keys,
            }
        }
    }

    /// Every key, each mode's in one keymap.
    fn bindings(&self) -> Vec<KeyBound<'_>> {
        self.keymap.bindings().collect()
    }

    /// Each mode's keys and the Search's, so the legend's chords follow too.
    fn rebind(&mut self, keys: &KeyBindingConfig) {
        (
            self.keymap,
            self.library_keys,
            self.inbox_keys,
            self.search_keys,
        ) = keymaps(keys);
    }

    /// The date style the page's dates follow.
    fn display(&mut self, display: &PageDisplay) {
        self.date_style = display.date_style;
    }

    /// A palette Command of the mode not showing shows that mode first.
    fn shows(&self, event: &DocumentsPageEvent) -> Option<DocumentsPageEvent> {
        event
            .mode()
            .filter(|mode| *mode != self.mode)
            .map(|mode| match mode {
                DocumentsMode::Inbox => DocumentsPageEvent::ShowInbox,
                DocumentsMode::Library => DocumentsPageEvent::ShowLibrary,
            })
    }

    /// In the Search, typing narrows live, `Backspace` edits and `Enter` keeps the query.
    fn typed(&self, key: TextKey) -> Option<DocumentsPageEvent> {
        if !self.searching {
            return None;
        }
        match key {
            TextKey::Char(ch) if !ch.is_control() => Some(DocumentsPageEvent::SearchType(ch)),
            TextKey::Backspace => Some(DocumentsPageEvent::SearchBackspace),
            TextKey::Enter { .. } => Some(DocumentsPageEvent::KeepSearch),
            TextKey::Char(_) | TextKey::Tab { .. } => None,
        }
    }

    /// The Library's Search is the page's Search mode.
    fn mode(&self) -> Option<PageMode> {
        self.searching.then_some(PageMode::Search)
    }
}

/// The page's keymaps under `config`: every key, the Library's, the Inbox's and the Search's.
fn keymaps(
    config: &KeyBindingConfig,
) -> (
    Keymap<DocumentsPageEvent>,
    Keymap<DocumentsPageEvent>,
    Keymap<DocumentsPageEvent>,
    Keymap<DocumentsPageEvent>,
) {
    let keymap = documents_page_keymap(config);
    let library = keymap.scoped(|event| event.mode() != Some(DocumentsMode::Inbox));
    let inbox = keymap.scoped(|event| {
        event.mode() != Some(DocumentsMode::Library) || *event == DocumentsPageEvent::Add
    });
    let search = keymap.scoped(|_| false);
    (keymap, library, inbox, search)
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    use chrono::{Duration, NaiveDate};
    use lib_config::KeyBindingConfig;
    use lib_core::PayeeId;

    use super::*;
    use crate::DocumentRepository as _;
    use crate::fake::{DocumentsCall, InMemoryDocuments, InMemoryFiles, RecordingDocumentsRouter};
    use crate::presenter::test_support::{locale, records, today};
    use crate::types::default_types;
    use crate::{KeyDate, KeyDateKind, YearFacet, sample_documents};

    const STRONG: &str = "IMG_4471.jpg";

    struct Page {
        store: Rc<RefCell<InMemoryDocuments>>,
        day: Rc<Cell<NaiveDate>>,
        presenter: DocumentsPagePresenter,
        router: RecordingDocumentsRouter,
    }

    fn page_with(files: InMemoryFiles, config: &KeyBindingConfig) -> Page {
        locale();
        let records = records(today());
        let store = Rc::new(RefCell::new(InMemoryDocuments {
            documents: sample_documents(&records, today()).documents,
            types: default_types(),
        }));
        let (clock, day) = Clock::settable(today());
        let shared: SharedRepository = store.clone();
        let presenter = DocumentsPagePresenter::new(
            shared,
            Rc::new(records),
            Rc::new(files),
            clock,
            config,
            "household.pldb".to_string(),
        );
        Page {
            store,
            day,
            presenter,
            router: RecordingDocumentsRouter::default(),
        }
    }

    fn page() -> Page {
        page_with(InMemoryFiles::default(), &KeyBindingConfig::default())
    }

    #[test]
    fn the_inbox_keeps_a_but_not_the_librarys_sort_and_the_library_has_no_inbox_keys() {
        let mut page = page();
        let keys = page.presenter.keymap();
        assert_eq!(keys.event("s"), Some(DocumentsPageEvent::Sort));
        assert_eq!(keys.event("y"), None, "Accept is the Inbox's");
        page.send(DocumentsPageEvent::ShowInbox);
        let keys = page.presenter.keymap();
        assert_eq!(
            keys.event("a"),
            Some(DocumentsPageEvent::Add),
            "swallowed, not Insert"
        );
        assert_eq!(keys.event("s"), None, "Sort is the Library's");
        assert_eq!(keys.event("y"), Some(DocumentsPageEvent::Accept));
    }

    #[test]
    fn a_command_of_the_other_mode_names_the_event_that_shows_that_mode() {
        let mut page = page();
        assert_eq!(page.presenter.shows(&DocumentsPageEvent::Add), None);
        assert_eq!(
            page.presenter.shows(&DocumentsPageEvent::AcceptAll),
            Some(DocumentsPageEvent::ShowInbox)
        );
        page.send(DocumentsPageEvent::ShowInbox);
        assert_eq!(
            page.presenter.shows(&DocumentsPageEvent::Add),
            Some(DocumentsPageEvent::ShowLibrary)
        );
        assert_eq!(page.presenter.shows(&DocumentsPageEvent::Delete), None);
    }

    #[test]
    fn the_search_is_the_pages_mode_and_takes_every_key_as_text() {
        let mut page = page();
        page.send(DocumentsPageEvent::StartSearch);
        assert_eq!(PagePresenter::mode(&page.presenter), Some(PageMode::Search));
        assert_eq!(page.presenter.keymap().event("j"), None, "j types");
        assert_eq!(
            page.presenter.typed(TextKey::Char('j')),
            Some(DocumentsPageEvent::SearchType('j'))
        );
        assert_eq!(
            page.presenter.typed(TextKey::Enter { shift: false }),
            Some(DocumentsPageEvent::KeepSearch)
        );
        page.send(DocumentsPageEvent::KeepSearch);
        assert_eq!(PagePresenter::mode(&page.presenter), None);
        assert_eq!(page.presenter.typed(TextKey::Char('j')), None);
    }

    impl Page {
        fn send(&mut self, event: DocumentsPageEvent) {
            self.presenter.handle(event, &mut self.router);
        }

        fn selected_file(&self) -> Option<String> {
            self.presenter
                .selected_document()
                .map(|document| document.file_name())
        }

        /// Moves the list selection down until it is on `file`.
        fn select(&mut self, file: &str) {
            self.send(DocumentsPageEvent::Move(Step::First));
            for _ in 0..self.presenter.row_count() {
                if self.selected_file().as_deref() == Some(file) {
                    return;
                }
                self.send(DocumentsPageEvent::Move(Step::Next));
            }
            assert_eq!(self.selected_file().as_deref(), Some(file));
        }

        fn inbox_rows(&self) -> Vec<InboxRowView> {
            self.presenter
                .view_model()
                .inbox
                .expect("the Inbox shows")
                .rows
        }

        fn library(&self) -> LibraryViewModel {
            self.presenter
                .view_model()
                .library
                .expect("the Library shows")
        }

        fn stored(&self, file: &str) -> Document {
            self.store
                .borrow()
                .documents
                .iter()
                .find(|document| document.file_name() == file)
                .cloned()
                .expect("the store holds that file")
        }

        fn toasts(&mut self) -> Vec<(ToastKind, String)> {
            self.router
                .take()
                .into_iter()
                .filter_map(|call| match call {
                    DocumentsCall::Notice(Notice::Toast(kind, text)) => Some((kind, text)),
                    _ => None,
                })
                .collect()
        }
    }

    #[test]
    fn the_page_opens_on_the_librarys_list_with_its_legend() {
        let page = page();
        let view = page.presenter.view_model();

        assert_eq!(view.mode, DocumentsMode::Library);
        assert_eq!(view.focus, DocumentsFocus::List);
        assert_eq!(view.title, "Documents");
        assert_eq!(view.rail_selected, RailEntry::Scope(LibraryScope::All));
        assert!(view.stored.ends_with("stored beside household.pldb"));
        let library = view.library.expect("the Library shows");
        assert!(!library.rows.is_empty());
        assert!(library.detail.is_some());
        assert!(
            library.count_line.ends_with("3 to review"),
            "{}",
            library.count_line
        );
        assert!(view.inbox.is_none());
        let keys: Vec<&str> = view.hints.iter().map(|hint| hint.chord.as_str()).collect();
        assert_eq!(keys, ["j/k", "enter", "/", "l", "L", "o", "i"]);
    }

    #[test]
    fn the_index_moves_the_scope_and_k_from_all_reaches_the_inbox() {
        let mut page = page();
        page.send(DocumentsPageEvent::FocusIndex);
        assert_eq!(page.presenter.focus(), DocumentsFocus::Index);

        page.send(DocumentsPageEvent::Move(Step::Next));
        assert!(matches!(page.presenter.scope(), LibraryScope::Type(_)));
        page.send(DocumentsPageEvent::Move(Step::First));
        assert_eq!(page.presenter.mode(), DocumentsMode::Inbox);
        page.send(DocumentsPageEvent::Move(Step::Last));
        assert!(matches!(
            page.presenter.scope(),
            LibraryScope::Year(YearFacet::Before(_))
        ));

        page.send(DocumentsPageEvent::Select);
        assert_eq!(page.presenter.focus(), DocumentsFocus::List);
        page.send(DocumentsPageEvent::Back);
        assert_eq!(page.presenter.focus(), DocumentsFocus::Index);
        page.send(DocumentsPageEvent::Link);
        assert_eq!(page.presenter.focus(), DocumentsFocus::List);
    }

    #[test]
    fn a_key_the_zone_or_mode_has_no_use_for_is_left_to_the_shell() {
        let mut page = page();
        assert!(!page.presenter.takes(&DocumentsPageEvent::Accept));
        assert!(!page.presenter.takes(&DocumentsPageEvent::Undo));
        assert!(!page.presenter.takes(&DocumentsPageEvent::FocusList));
        assert!(page.presenter.takes(&DocumentsPageEvent::Sort));

        page.send(DocumentsPageEvent::ToggleMode);
        assert!(page.presenter.takes(&DocumentsPageEvent::Accept));
        assert!(!page.presenter.takes(&DocumentsPageEvent::Sort));
        assert!(
            page.presenter.takes(&DocumentsPageEvent::Add),
            "a is swallowed in the Inbox so it never reaches the Shell's Insert"
        );
        page.send(DocumentsPageEvent::FocusIndex);
        assert!(!page.presenter.takes(&DocumentsPageEvent::FocusIndex));
        assert!(!page.presenter.takes(&DocumentsPageEvent::Skip));
    }

    #[test]
    fn the_sort_toggles_and_puts_the_selection_back_at_the_top() {
        let mut page = page();
        page.send(DocumentsPageEvent::Move(Step::Next));

        page.send(DocumentsPageEvent::Sort);

        assert_eq!(page.presenter.sort(), LibrarySort::Expiring);
        assert_eq!(page.presenter.selected(), 0);
        let first = &page.library().rows[0];
        assert!(
            matches!(first.tail, library::RowTail::Flag { red: true, .. }),
            "Expiring leads with a Need Review row"
        );
    }

    #[test]
    fn the_search_narrows_live_and_esc_clears_it() {
        let mut page = page();
        let all = page.library().rows.len();

        page.send(DocumentsPageEvent::StartSearch);
        assert!(page.presenter.searching());
        for ch in "passport".chars() {
            page.send(DocumentsPageEvent::SearchType(ch));
        }
        assert_eq!(page.library().rows.len(), 1);
        page.send(DocumentsPageEvent::KeepSearch);
        assert!(!page.presenter.searching());
        assert_eq!(page.presenter.query(), "passport");
        for _ in 0..8 {
            page.send(DocumentsPageEvent::SearchType('z'));
        }
        assert_eq!(
            page.presenter.query(),
            "passport",
            "typing needs the Search open"
        );

        page.send(DocumentsPageEvent::Back);
        assert_eq!(page.presenter.query(), "");
        assert_eq!(page.library().rows.len(), all);
        assert_eq!(page.presenter.focus(), DocumentsFocus::List);

        page.send(DocumentsPageEvent::ShowInbox);
        page.send(DocumentsPageEvent::StartSearch);
        assert!(!page.presenter.searching(), "the Inbox has no Search");
    }

    #[test]
    fn accept_files_the_strong_row_and_undo_puts_it_back_as_one_unit() {
        let mut page = page();
        page.send(DocumentsPageEvent::ShowInbox);
        page.select(STRONG);
        let before = page.inbox_rows().len();

        page.send(DocumentsPageEvent::Accept);

        assert_eq!(page.inbox_rows().len(), before - 1);
        let filed = page.stored(STRONG);
        assert!(filed.is_filed(), "Accept writes the Document Filed");
        assert_eq!(
            filed.links,
            [DocumentLink::Transaction(TransactionId::seed(1))]
        );
        assert_ne!(page.selected_file().as_deref(), Some(STRONG));
        assert_eq!(
            page.toasts(),
            [(
                ToastKind::Success,
                "Filed \"IMG_4471\" · u undo".to_string()
            )]
        );

        page.send(DocumentsPageEvent::Undo);

        assert_eq!(page.inbox_rows().len(), before);
        assert!(page.stored(STRONG).is_unfiled());
        assert!(page.stored(STRONG).links.is_empty());
        assert_eq!(page.selected_file().as_deref(), Some(STRONG));
        assert_eq!(
            page.toasts(),
            [(ToastKind::Info, "Put 1 back in the Inbox.".to_string())]
        );
        page.send(DocumentsPageEvent::Undo);
        assert_eq!(
            page.router.take(),
            [DocumentsCall::Notice(Notice::Status(
                "Nothing to undo.".to_string()
            ))],
            "undo is one level"
        );
    }

    #[test]
    fn accept_on_a_row_with_nothing_to_suggest_opens_the_filing_picker() {
        let mut page = page();
        page.send(DocumentsPageEvent::ShowInbox);
        page.select("scan0012.pdf");
        let id = page.stored("scan0012.pdf").id;

        page.send(DocumentsPageEvent::Accept);

        assert_eq!(
            page.router.take(),
            [DocumentsCall::OpenDialog(DocumentsDialog::File(id))]
        );
        page.send(DocumentsPageEvent::DialogClosed(Some(
            DocumentsOutcome::FileByHand {
                id,
                doc_type: DocumentType::TAX,
                link: None,
            },
        )));
        let filed = page.stored("scan0012.pdf");
        assert!(filed.is_filed());
        assert_eq!(filed.doc_type, DocumentType::TAX);
        page.send(DocumentsPageEvent::Undo);
        assert!(
            page.stored("scan0012.pdf").is_unfiled(),
            "a filing by hand is undone too"
        );
    }

    #[test]
    fn skip_sinks_the_row_writes_nothing_and_is_gone_on_a_restart() {
        let mut page = page();
        page.send(DocumentsPageEvent::ShowInbox);
        page.select(STRONG);
        let at = page.presenter.selected();
        let stored = page.store.borrow().clone();

        page.send(DocumentsPageEvent::Skip);

        let rows = page.inbox_rows();
        assert_eq!(rows.last().map(|row| row.name.as_str()), Some(STRONG));
        assert!(rows.last().is_some_and(|row| row.skipped));
        assert_eq!(
            page.presenter.selected(),
            at,
            "focus stays, now on the next row"
        );
        assert_eq!(*page.store.borrow(), stored, "Skip writes nothing");

        let shared: SharedRepository = page.store.clone();
        let restarted = DocumentsPagePresenter::new(
            shared,
            Rc::new(records(today())),
            Rc::new(InMemoryFiles::default()),
            Clock::fixed(today()),
            &KeyBindingConfig::default(),
            String::new(),
        );
        assert!(restarted.skipped().is_empty());
        assert!(!restarted.view_model().rail.is_empty());
    }

    #[test]
    fn facts_saved_clears_the_skip_and_follows_the_document() {
        let mut page = page();
        page.send(DocumentsPageEvent::ShowInbox);
        page.select(STRONG);
        let id = page.stored(STRONG).id;
        page.send(DocumentsPageEvent::Skip);

        page.send(DocumentsPageEvent::DialogClosed(Some(
            DocumentsOutcome::FactsSaved(id),
        )));

        assert!(page.presenter.skipped().is_empty());
        assert_eq!(page.selected_file().as_deref(), Some(STRONG));
    }

    #[test]
    fn accept_all_asks_with_the_count_and_its_batch_is_one_undo() {
        let mut page = page();
        page.send(DocumentsPageEvent::ShowInbox);

        page.send(DocumentsPageEvent::AcceptAll);
        assert_eq!(
            page.router.take(),
            [DocumentsCall::OpenDialog(DocumentsDialog::AcceptAll(1))]
        );

        page.send(DocumentsPageEvent::DialogClosed(Some(
            DocumentsOutcome::AcceptAll,
        )));
        assert_eq!(
            page.toasts(),
            [(ToastKind::Success, "Filed 1 · u undo".to_string())]
        );
        page.send(DocumentsPageEvent::AcceptAll);
        assert_eq!(
            page.router.take(),
            [DocumentsCall::Notice(Notice::Status(
                "No strong matches to accept.".to_string()
            ))]
        );

        page.send(DocumentsPageEvent::Undo);
        assert!(page.stored(STRONG).is_unfiled());
    }

    #[test]
    fn delete_opens_the_confirm_on_the_selected_document_in_both_modes() {
        let mut page = page();
        page.select("aami-car-policy.pdf");
        let filed = page.stored("aami-car-policy.pdf").id;
        page.send(DocumentsPageEvent::Delete);
        page.send(DocumentsPageEvent::ShowInbox);
        page.select(STRONG);
        let unfiled = page.stored(STRONG).id;
        page.send(DocumentsPageEvent::Delete);

        assert_eq!(
            page.router.take(),
            [
                DocumentsCall::OpenDialog(DocumentsDialog::Delete(filed)),
                DocumentsCall::OpenDialog(DocumentsDialog::Delete(unfiled)),
            ]
        );
        assert!(page.presenter.takes(&DocumentsPageEvent::Delete));
        assert_eq!(
            page.store.borrow().documents.len(),
            sample_documents(&records(today()), today()).documents.len(),
            "opening the confirm deletes nothing"
        );
    }

    #[test]
    fn delete_with_no_document_selected_says_so() {
        let mut page = page();
        page.store.borrow_mut().documents.clear();

        page.send(DocumentsPageEvent::Delete);

        assert_eq!(
            page.router.take(),
            [DocumentsCall::Notice(Notice::Status(
                "No document is selected.".to_string()
            ))]
        );
    }

    /// What the Dialog does before it closes: removes the Document, then tells the page.
    fn delete(page: &mut Page, file: &str) {
        delete_id(page, page.stored(file).id);
    }

    fn delete_id(page: &mut Page, id: DocumentId) {
        page.store.borrow_mut().delete_document(id);
        page.send(DocumentsPageEvent::DialogClosed(Some(
            DocumentsOutcome::Deleted(id),
        )));
    }

    #[test]
    fn a_deleted_inbox_row_hands_focus_to_the_next_row_else_the_previous() {
        let mut page = page();
        page.send(DocumentsPageEvent::ShowInbox);
        let names: Vec<String> = page.inbox_rows().into_iter().map(|row| row.name).collect();
        assert!(names.len() >= 3);

        page.send(DocumentsPageEvent::Move(Step::First));
        delete(&mut page, &names[0]);
        assert_eq!(page.selected_file().as_deref(), Some(names[1].as_str()));

        page.send(DocumentsPageEvent::Move(Step::Last));
        delete(&mut page, &names[names.len() - 1]);
        assert_eq!(
            page.selected_file().as_deref(),
            Some(names[names.len() - 2].as_str())
        );
    }

    #[test]
    fn a_deleted_skipped_document_leaves_the_skipped_set() {
        let mut page = page();
        page.send(DocumentsPageEvent::ShowInbox);
        page.select(STRONG);
        page.send(DocumentsPageEvent::Skip);
        assert_eq!(page.presenter.skipped().len(), 1);

        delete(&mut page, STRONG);

        assert!(page.presenter.skipped().is_empty());
        assert!(page.inbox_rows().iter().all(|row| row.name != STRONG));
    }

    #[test]
    fn a_deleted_library_row_keeps_the_scope_even_when_it_is_left_empty() {
        let mut page = page();
        let scope = {
            page.send(DocumentsPageEvent::FocusIndex);
            page.send(DocumentsPageEvent::Move(Step::Next));
            page.presenter.scope()
        };
        assert!(matches!(scope, LibraryScope::Type(_)));
        page.send(DocumentsPageEvent::FocusList);
        let ids: Vec<DocumentId> = page.library().rows.into_iter().map(|row| row.id).collect();
        for id in ids {
            delete_id(&mut page, id);
        }

        assert_eq!(page.presenter.scope(), scope);
        assert!(page.library().rows.is_empty());
        assert_eq!(page.presenter.selected(), 0);
    }

    #[test]
    fn deleting_the_document_a_filing_undo_names_clears_the_undo() {
        let mut page = page();
        page.send(DocumentsPageEvent::ShowInbox);
        page.select(STRONG);
        page.send(DocumentsPageEvent::Accept);
        page.router.take();

        delete(&mut page, STRONG);
        page.send(DocumentsPageEvent::Undo);

        assert_eq!(
            page.router.take(),
            [DocumentsCall::Notice(Notice::Status(
                "Nothing to undo.".to_string()
            ))],
            "`u` never reports undoing a Document that is gone"
        );
    }

    #[test]
    fn deleting_an_unrelated_document_leaves_the_undo_alone() {
        let mut page = page();
        page.send(DocumentsPageEvent::ShowInbox);
        page.select(STRONG);
        page.send(DocumentsPageEvent::Accept);
        page.router.take();

        delete(&mut page, "scan0012.pdf");
        page.send(DocumentsPageEvent::Undo);

        assert!(page.stored(STRONG).is_unfiled());
    }

    #[test]
    fn follow_reveals_one_link_lists_several_and_reveals_the_inboxs_suggestion() {
        let mut page = page();
        page.select("amex-platinum-statement.pdf");
        let one = page.stored("amex-platinum-statement.pdf").links;
        assert_eq!(one.len(), 1);
        page.send(DocumentsPageEvent::Follow);
        assert_eq!(page.router.take(), [DocumentsCall::Reveal(one[0])]);

        page.select("aami-car-policy.pdf");
        let id = page.stored("aami-car-policy.pdf").id;
        page.send(DocumentsPageEvent::Follow);
        assert_eq!(
            page.router.take(),
            [DocumentsCall::OpenDialog(DocumentsDialog::Follow(id))]
        );
        page.send(DocumentsPageEvent::DialogClosed(Some(
            DocumentsOutcome::Follow(DocumentLink::Payee(PayeeId::seed(2))),
        )));
        assert_eq!(
            page.router.take(),
            [DocumentsCall::Reveal(DocumentLink::Payee(PayeeId::seed(2)))]
        );

        page.send(DocumentsPageEvent::ShowInbox);
        page.select(STRONG);
        page.send(DocumentsPageEvent::Follow);
        assert_eq!(
            page.router.take(),
            [DocumentsCall::Reveal(DocumentLink::Transaction(
                TransactionId::seed(1)
            ))]
        );
    }

    #[test]
    fn a_link_chip_reveals_its_record_and_its_cross_removes_it_with_a_toast() {
        let mut page = page();
        page.select("amex-platinum-statement.pdf");
        let detail = page.library().detail.expect("a detail pane");
        let link = detail.links[0].link;
        assert_eq!(detail.links[0].kind, "Account");

        page.send(DocumentsPageEvent::Reveal(link));
        assert_eq!(page.router.take(), [DocumentsCall::Reveal(link)]);

        page.send(DocumentsPageEvent::Unlink(link));
        assert!(page.stored("amex-platinum-statement.pdf").links.is_empty());
        assert!(page.stored("amex-platinum-statement.pdf").is_filed());
        assert_eq!(
            page.toasts(),
            [(ToastKind::Info, "Unlinked Amex Platinum.".to_string())]
        );
        assert_eq!(page.presenter.selected_chips(), Vec::<String>::new());
    }

    #[test]
    fn a_missing_file_says_so_and_one_on_disk_goes_to_the_platform() {
        let mut page = page();
        page.select("amex-platinum-statement.pdf");
        page.send(DocumentsPageEvent::ShowInFolder);
        let calls = page.router.take();
        assert!(matches!(
            calls.as_slice(),
            [DocumentsCall::Notice(Notice::Toast(ToastKind::Warning, text))]
                if text.starts_with("amex-platinum-statement.pdf is not at /work/")
        ));

        let path = page.stored("amex-platinum-statement.pdf").path;
        let on_disk = Path::new("/work").join(&path);
        let mut page = page_with(
            InMemoryFiles::default().with(on_disk.clone(), 1),
            &KeyBindingConfig::default(),
        );
        page.select("amex-platinum-statement.pdf");
        page.send(DocumentsPageEvent::Select);
        assert_eq!(
            page.router.take(),
            [
                DocumentsCall::OpenFile {
                    path: on_disk,
                    in_folder: false
                },
                DocumentsCall::Notice(Notice::Status(
                    "Opened amex-platinum-statement.pdf.".to_string()
                ))
            ]
        );
    }

    #[test]
    fn a_drop_lands_what_exists_in_the_inbox_and_the_page_stays() {
        let files = InMemoryFiles::default().with("/work/new-scan.pdf", 4);
        let mut page = page_with(files, &KeyBindingConfig::default());
        let inbox = page
            .store
            .borrow()
            .documents
            .iter()
            .filter(|d| d.is_unfiled())
            .count();

        page.send(DocumentsPageEvent::Drop(vec![
            "/work/new-scan.pdf".to_string(),
            "/work/gone.pdf".to_string(),
        ]));

        assert_eq!(
            page.store
                .borrow()
                .documents
                .iter()
                .filter(|d| d.is_unfiled())
                .count(),
            inbox + 1
        );
        assert_eq!(
            page.stored("new-scan.pdf").intake.map(|i| i.source),
            Some(Source::Dropped)
        );
        assert_eq!(page.presenter.mode(), DocumentsMode::Library);
        assert_eq!(
            page.toasts(),
            [(
                ToastKind::Success,
                "1 imported, 1 not imported.".to_string()
            )]
        );
        page.send(DocumentsPageEvent::Drop(vec!["/work/gone.pdf".to_string()]));
        assert_eq!(
            page.toasts(),
            [(ToastKind::Error, "0 imported, 1 not imported.".to_string())]
        );
    }

    #[test]
    fn edit_and_the_picker_open_on_the_selection_by_mode() {
        let mut page = page();
        page.select("amex-platinum-statement.pdf");
        let filed = page.stored("amex-platinum-statement.pdf").id;
        page.send(DocumentsPageEvent::Edit);
        page.send(DocumentsPageEvent::Link);
        page.send(DocumentsPageEvent::Add);
        assert_eq!(
            page.router.take(),
            [
                DocumentsCall::OpenDialog(DocumentsDialog::Edit(filed)),
                DocumentsCall::OpenDialog(DocumentsDialog::Link(filed)),
                DocumentsCall::OpenDialog(DocumentsDialog::Add(LibraryScope::All)),
            ]
        );

        page.send(DocumentsPageEvent::ShowInbox);
        page.select(STRONG);
        let unfiled = page.stored(STRONG).id;
        page.send(DocumentsPageEvent::Edit);
        page.send(DocumentsPageEvent::Link);
        page.send(DocumentsPageEvent::Add);
        assert_eq!(
            page.router.take(),
            [
                DocumentsCall::OpenDialog(DocumentsDialog::Facts(unfiled)),
                DocumentsCall::OpenDialog(DocumentsDialog::File(unfiled)),
            ],
            "Add does nothing in the Inbox"
        );
    }

    #[test]
    fn a_document_added_or_edited_shows_on_all_documents() {
        let mut page = page();
        page.send(DocumentsPageEvent::ClickRail(RailEntry::Scope(
            LibraryScope::Type(DocumentType::TAX),
        )));
        let passport = page
            .store
            .borrow()
            .documents
            .iter()
            .find(|d| d.title == "Passport — scan")
            .map(|d| d.id)
            .expect("the passport");

        page.send(DocumentsPageEvent::DialogClosed(Some(
            DocumentsOutcome::Shown(passport),
        )));

        assert_eq!(page.presenter.scope(), LibraryScope::All);
        assert_eq!(page.presenter.focus(), DocumentsFocus::List);
        assert_eq!(
            page.presenter.selected_document().map(|d| d.id),
            Some(passport)
        );
    }

    #[test]
    fn the_day_moving_on_moves_a_key_date_into_need_review() {
        let mut page = page();
        let id = page.stored("aami-car-policy.pdf").id;
        let mut document = page.stored("aami-car-policy.pdf");
        document.key_date = Some(KeyDate {
            kind: KeyDateKind::Renews,
            date: today() + Duration::days(70),
            reminder: false,
        });
        page.store.borrow_mut().documents.retain(|d| d.id != id);
        page.store.borrow_mut().documents.push(document);
        let review = |page: &Page| {
            page.library()
                .rows
                .iter()
                .find(|row| row.id == id)
                .map(|row| row.tail.clone())
        };
        assert!(matches!(
            review(&page),
            Some(library::RowTail::Flag { red: false, .. })
        ));

        page.day.set(today() + Duration::days(10));
        page.send(DocumentsPageEvent::MonthTurned);

        assert!(matches!(
            review(&page),
            Some(library::RowTail::Flag { red: true, .. })
        ));
    }

    #[test]
    fn a_scope_that_leaves_the_rail_falls_back_to_all_documents() {
        let mut page = page();
        page.send(DocumentsPageEvent::ClickRail(RailEntry::Scope(
            LibraryScope::Year(YearFacet::Before(2025)),
        )));
        page.send(DocumentsPageEvent::ClickRail(RailEntry::Scope(
            LibraryScope::Year(YearFacet::Year(2025)),
        )));

        // The Financial Year turns on 1 July: 2025–26 is now last year's, and Before 2025 is gone.
        page.day
            .set(NaiveDate::from_ymd_opt(2027, 7, 1).expect("a real date"));
        page.send(DocumentsPageEvent::MonthTurned);
        assert_eq!(page.presenter.scope(), LibraryScope::All);

        let tax = DocumentType::TAX.0;
        page.send(DocumentsPageEvent::ClickRail(RailEntry::Scope(
            LibraryScope::Type(DocumentType::TAX),
        )));
        page.store.borrow_mut().types.retain(|row| row.id != tax);
        assert_eq!(page.presenter.scope(), LibraryScope::All);
        assert_eq!(
            page.presenter.view_model().rail_selected,
            RailEntry::Scope(LibraryScope::All)
        );
    }

    #[test]
    fn the_legend_and_the_cheat_sheet_follow_the_keymap() {
        let mut config = KeyBindingConfig::default();
        config
            .bindings
            .insert(ACCEPT.to_string(), "ctrl+y".to_string());
        let mut page = page_with(InMemoryFiles::default(), &config);
        page.send(DocumentsPageEvent::ShowInbox);

        let view = page.presenter.view_model();
        assert!(view.hints.contains(&Hint::new("ctrl+y", "accept")));
        assert_eq!(view.inbox.expect("the Inbox").keys, view.hints);
        let help = page.presenter.help();
        for hint in &view.hints {
            assert!(
                help.contains(hint),
                "{hint:?} is missing from the cheat sheet"
            );
        }
        assert!(help.contains(&Hint::new("Y", "accept all strong matches")));
        assert!(help.contains(&Hint::new("I", "import")));
    }
}
