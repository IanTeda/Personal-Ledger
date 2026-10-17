//! The Documents destination's Entities (ADR-0032, ADR-0037). The Documents and the Document Types
//! are read through `LedgerStore`; Settings › Documents edits the Types too.
//! [`DocumentsView`] owns the page's own state: mode, scope, sort, search, focus zone, selections
//! and the last filing for `u`. It renders itself (`impl Render` in `super`), handles its own keys,
//! `Esc`, Search and movement, files Documents, hands files to the OS, and reaches `Shell` only
//! through [`ViewEvent`]. It builds the Documents dialogs, which `Shell`'s Dialog host draws and
//! confirms.

use chrono::NaiveDate;
use gpui::{
    App, Context, Entity, EventEmitter, Keystroke, ScrollHandle, ScrollStrategy, Subscription,
    UniformListScrollHandle,
};
use lib_core::{DateStyle, DocumentId, DocumentTypeId, TransactionId};
use lib_ledger::{
    AcceptDocument, FileDocumentByHand, LedgerCommand, SkipDocument, ToggleDocumentLink, UndoFiling,
};
use lib_toast::ToastKind;

use super::{
    DocumentsFocus,
    hints::{inbox_hints, index_hints, library_hints},
};
use crate::{
    chrome::{dialog_host::OpenDialog, statusline::PageStatus},
    documents::{
        self, Document, DocumentLink, DocumentsMode, FilingUndo, KeyDateKind, LibraryScope,
        LibrarySort, RailEntry,
        form::{DocumentForm, DocumentOptions, DocumentsDialog, FactsForm, ImportForm},
        model::Lookups,
        picker::{PickerRow, PickerState, Purpose},
        types::DocumentTypeRow,
    },
    ledger_store::{LedgerStore, notify_on},
    navigation::{key_router::Movement, nav::Noun},
    view::event::{Reveal, ViewEvent, ViewMode},
};
use lib_ledger::Domain;

impl EventEmitter<ViewEvent> for DocumentsView {}

/// The Documents page's own state, owned by its Entity.
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
    /// The Library list's scroll: a `uniform_list`, so only the rows in view are built.
    pub library_scroll: UniformListScrollHandle,
    /// The Inbox list's scroll. Its rows vary in height, so it stays a plain scrolling `div`.
    pub inbox_scroll: ScrollHandle,
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
            library_scroll: UniformListScrollHandle::new(),
            inbox_scroll: ScrollHandle::new(),
            inbox_selected: 0,
            undo: None,
        }
    }
}

/// The Ledger the Documents page reads, its Inventory Links included, and writes the Documents
/// through.
#[derive(Clone)]
pub struct DocumentsSources {
    pub ledger: Entity<LedgerStore>,
}

/// The Documents Entity.
pub struct DocumentsView {
    pub(super) sources: DocumentsSources,
    pub(super) today: NaiveDate,
    /// The Display Preference the dates on the page and in its forms follow. Pushed by `Shell`,
    /// which owns the Settings page that changes it.
    pub(super) date_style: Option<DateStyle>,
    /// Whether the View zone has focus, which the index rail's and the list's focus rings draw
    /// within. Handed over by `Shell` as it draws the page; the View never reads `Shell`'s focus.
    pub(super) zone_focused: bool,
    /// Whether the Library's Search is open (`InputMode::Search`, which the View entered).
    pub(super) searching: bool,
    pub(super) state: DocumentsState,
    /// Repaints the page when the Accounts its rows and filters name change.
    _accounts_changed: Subscription,
    /// Repaints the page when the Payees its rows and filters name change.
    _payees_changed: Subscription,
}

/// The Key date select's label for each kind.
fn key_label_for_options(kind: Option<KeyDateKind>) -> String {
    match kind {
        None => crate::msg::desktop_documents_key_none(),
        Some(KeyDateKind::Renews) => crate::msg::desktop_documents_detail_key_renews(),
        Some(KeyDateKind::Ends) => crate::msg::desktop_documents_detail_key_ends(),
        Some(KeyDateKind::Expires) => crate::msg::desktop_documents_detail_key_expires(),
        Some(KeyDateKind::Revalue) => crate::msg::desktop_documents_detail_key_revalue(),
    }
}

impl DocumentsView {
    pub fn new(sources: DocumentsSources, today: NaiveDate, cx: &mut Context<'_, Self>) -> Self {
        let _accounts_changed = notify_on(&sources.ledger, Domain::Accounts, cx);
        let _payees_changed = notify_on(&sources.ledger, Domain::Payees, cx);
        Self {
            sources,
            today,
            date_style: None,
            zone_focused: false,
            searching: false,
            state: DocumentsState::default(),
            _accounts_changed,
            _payees_changed,
        }
    }

    pub fn state(&self) -> &DocumentsState {
        &self.state
    }

    /// Follows a change to the Display Preferences.
    pub fn set_display(&mut self, date_style: Option<DateStyle>, cx: &mut Context<'_, Self>) {
        self.date_style = date_style;
        cx.notify();
    }

    /// Records whether the View zone has focus, as `Shell` draws the page.
    pub fn set_zone_focused(&mut self, focused: bool) {
        self.zone_focused = focused;
    }

    /// The mode, scope and sort that persist across restarts.
    pub fn persisted(&self) -> (DocumentsMode, LibraryScope, LibrarySort) {
        (self.state.mode, self.state.scope, self.state.sort)
    }

    /// Restores the mode, scope and sort from the last run.
    pub fn set_persisted(
        &mut self,
        mode: DocumentsMode,
        scope: LibraryScope,
        sort: LibrarySort,
        cx: &mut Context<'_, Self>,
    ) {
        self.state.mode = mode;
        self.state.scope = scope;
        self.state.sort = sort;
        cx.notify();
    }

    pub(super) fn documents<'a>(&self, cx: &'a App) -> &'a [Document] {
        self.sources.ledger.read(cx).ledger().documents()
    }

    pub(super) fn types<'a>(&self, cx: &'a App) -> &'a [DocumentTypeRow] {
        self.sources.ledger.read(cx).ledger().document_types()
    }

    /// Runs `command` through the Ledger, which announces the Documents it changed.
    fn apply<C: LedgerCommand>(&self, cx: &mut App, command: C) -> Result<C::Outcome, C::Refusal> {
        self.sources
            .ledger
            .update(cx, |store, cx| store.apply(cx, command))
    }

    /// Everything a Document's rows, detail and picker rows are worded from.
    pub fn lookups<'a>(&self, cx: &'a App) -> Lookups<'a> {
        Lookups {
            accounts: self.sources.ledger.read(cx).ledger().accounts(),
            payees: self.sources.ledger.read(cx).ledger().payees(),
            plans: self.sources.ledger.read(cx).ledger().bill_plans(),
            types: self.types(cx),
            inventory: self.sources.ledger.read(cx).ledger().inventory(),
            transactions: self.sources.ledger.read(cx).ledger().transactions(),
            today: self.today,
            date_style: self.date_style,
        }
    }

    fn options(&self, cx: &App) -> DocumentOptions {
        DocumentOptions::new(self.types(cx), key_label_for_options)
    }

    /// The Library's rows under the current scope, search and sort.
    pub fn library_rows<'a>(&self, cx: &'a App) -> Vec<&'a Document> {
        documents::library_rows(
            self.types(cx),
            self.documents(cx),
            self.state.scope,
            &self.state.query,
            self.state.sort,
            self.today,
        )
    }

    /// The Inbox's rows: newest received first, Skipped Documents last.
    pub fn inbox_rows<'a>(&self, cx: &'a App) -> Vec<&'a Document> {
        documents::inbox_rows(self.documents(cx))
    }

    /// The Inbox row focus is on, a position clamped to the rows.
    pub(super) fn inbox_at(&self, len: usize) -> usize {
        self.state.inbox_selected.min(len.saturating_sub(1))
    }

    pub(super) fn rail_selected(&self) -> RailEntry {
        match self.state.mode {
            DocumentsMode::Inbox => RailEntry::Inbox,
            DocumentsMode::Library => RailEntry::Scope(self.state.scope),
        }
    }

    /// The Document the selection is on: the Library's row or the Inbox's focused row.
    pub fn selected_document<'a>(&self, cx: &'a App) -> Option<&'a Document> {
        match self.state.mode {
            DocumentsMode::Library => {
                let rows = self.library_rows(cx);
                let at = self.state.selected.min(rows.len().saturating_sub(1));
                rows.get(at).copied()
            }
            DocumentsMode::Inbox => {
                let rows = self.inbox_rows(cx);
                rows.get(self.inbox_at(rows.len())).copied()
            }
        }
    }

    /// "1.8 GB · stored beside household.pldb": the summed stub file sizes and the Ledger file.
    pub(super) fn stored_text(&self, cx: &App) -> String {
        crate::msg::desktop_documents_status_stored(
            &documents::model::size_text(documents::total_bytes(self.documents(cx))),
            crate::chrome::statusline::STUB_LEDGER_FILE,
        )
    }

    // -----------------------------------------------------------------------------------------
    // The interface `Shell` forwards to (ADR-0037)
    // -----------------------------------------------------------------------------------------

    /// The Documents keys, ahead of the router (`a` would otherwise enter Insert mode). `false`
    /// for any key the page does not own.
    pub fn key_ahead(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        let modifiers = &keystroke.modifiers;
        if modifiers.function {
            return false;
        }
        let shift = modifiers.shift;
        let library = self.state.mode == DocumentsMode::Library;
        let in_list = self.state.focus == DocumentsFocus::List;
        match keystroke.key.as_str() {
            "i" if shift => self.open_import(cx),
            "i" => self.toggle_mode(cx),
            // Library only; swallowed in the Inbox so `a` does not enter Insert mode there.
            "a" if !shift => {
                if library {
                    self.open_add(cx);
                }
            }
            "s" if library && !shift => {
                self.state.sort = self.state.sort.toggle();
                self.reset_selection();
            }
            "e" if !shift => {
                if library {
                    self.open_edit(cx);
                } else {
                    self.open_facts(cx);
                }
            }
            "l" | "right" if !in_list => self.state.focus = DocumentsFocus::List,
            "l" if in_list && !shift => self.link_picker(cx),
            "l" if in_list && shift => self.follow_link(cx),
            "h" | "left" if in_list => self.state.focus = DocumentsFocus::Index,
            "o" if in_list => self.open_selected(true, cx),
            // Inbox only; in the Library these fall through to the global router.
            "y" if !library && shift => self.open_accept_all(cx),
            "y" if !library && in_list => self.accept_focused(cx),
            "x" if !library && in_list && !shift => self.skip_focused(cx),
            "u" if !library && !shift => self.undo_last(cx),
            _ => return false,
        }
        cx.notify();
        true
    }

    /// No Documents key waits for the router's leftovers: the page claims its keys ahead of it.
    pub fn handle_key(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// `j`/`k`/`gg`/`G`/`ctrl-d`/`ctrl-u`/`enter`, by focus zone.
    pub fn apply_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) {
        match self.state.focus {
            DocumentsFocus::Index => self.apply_rail_movement(movement, cx),
            DocumentsFocus::List => self.apply_list_movement(movement, cx),
        }
        cx.notify();
    }

    /// `Esc`, innermost first: the Search (its query cleared), then a committed query, then the
    /// list back to the index.
    pub fn escape(&mut self, cx: &mut Context<'_, Self>) -> bool {
        if self.searching {
            self.state.query.clear();
            self.reset_selection();
            self.stop_search(cx);
        } else if !self.state.query.is_empty() {
            self.state.query.clear();
            self.reset_selection();
        } else if self.state.focus == DocumentsFocus::List {
            self.state.focus = DocumentsFocus::Index;
        } else {
            return false;
        }
        cx.notify();
        true
    }

    /// The Library's `/`: focuses the toolbar search from either zone. Inert in the Inbox, which
    /// draws no search box.
    pub fn start_search(&mut self, cx: &mut Context<'_, Self>) -> bool {
        if self.state.mode != DocumentsMode::Library {
            return false;
        }
        self.state.focus = DocumentsFocus::List;
        self.searching = true;
        cx.notify();
        true
    }

    /// Keys while the Search is open: typing narrows live, `Backspace` edits, `Enter` keeps the
    /// query and returns to the list. `Esc` comes back as [`Self::escape`].
    pub fn search_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        match keystroke.key.as_str() {
            "enter" => self.stop_search(cx),
            "backspace" => {
                self.state.query.pop();
                self.reset_selection();
            }
            _ => {
                let modifiers = &keystroke.modifiers;
                if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                    return false;
                }
                let mut chars = keystroke.key_char.as_deref().unwrap_or_default().chars();
                match (chars.next(), chars.next()) {
                    (Some(ch), None) if !ch.is_control() => {
                        self.state.query.push(ch);
                        self.reset_selection();
                    }
                    _ => return false,
                }
            }
        }
        cx.notify();
        true
    }

    /// The page has just become the destination: `g f` and the palette land on the list, unlike
    /// Settings' index. A Search left open does not come back with it.
    pub fn on_enter(&mut self, cx: &mut Context<'_, Self>) {
        self.state.focus = DocumentsFocus::List;
        if self.searching {
            self.stop_search(cx);
        }
        cx.notify();
    }

    /// No other View hands a record to Documents yet.
    pub fn reveal(&mut self, _reveal: Reveal, _cx: &mut Context<'_, Self>) {}

    /// The status line's legend for the focus zone and mode, and the stored-size text.
    pub fn page_status(&self, cx: &App) -> Option<PageStatus> {
        let hints = if self.state.focus == DocumentsFocus::Index {
            index_hints()
        } else if self.state.mode == DocumentsMode::Inbox {
            inbox_hints()
        } else {
            library_hints()
        };
        Some(PageStatus {
            hints,
            right: self.stored_text(cx),
        })
    }

    /// The `?` cheat-sheet's Documents group as `(action, keys)`: the index keys, then the current
    /// mode's.
    pub fn cheat_sheet(&self) -> Vec<(String, &'static str)> {
        let mode = match self.state.mode {
            DocumentsMode::Library => {
                let mut hints = library_hints();
                hints.push(("s", crate::msg::desktop_hint_sort()));
                hints.push(("e", crate::msg::desktop_hint_edit()));
                hints.push(("a", crate::msg::desktop_hint_add()));
                hints
            }
            DocumentsMode::Inbox => {
                let mut hints = inbox_hints();
                hints.push(("u", crate::msg::desktop_hint_undo()));
                hints.push(("Y", crate::msg::desktop_hint_accept_all()));
                hints
            }
        };
        index_hints()
            .into_iter()
            .chain(mode)
            .chain(std::iter::once(("I", crate::msg::desktop_hint_import())))
            .map(|(keys, action)| (action, keys))
            .collect()
    }

    // -----------------------------------------------------------------------------------------
    // Navigation: mode, scope, focus, selection
    // -----------------------------------------------------------------------------------------

    /// Switches Inbox / Library, putting the list back at its top when the mode changes.
    pub fn set_mode(&mut self, mode: DocumentsMode, cx: &mut Context<'_, Self>) {
        if self.state.mode != mode {
            self.state.mode = mode;
            self.state
                .library_scroll
                .scroll_to_item(0, ScrollStrategy::Top);
            self.state.inbox_scroll.set_offset(gpui::Point::default());
        }
        cx.notify();
    }

    /// `i`: toggles Inbox and Library, keeping focus in its zone.
    fn toggle_mode(&mut self, cx: &mut Context<'_, Self>) {
        self.set_mode(
            match self.state.mode {
                DocumentsMode::Library => DocumentsMode::Inbox,
                DocumentsMode::Inbox => DocumentsMode::Library,
            },
            cx,
        );
    }

    fn reset_selection(&mut self) {
        self.state.selected = 0;
        self.state
            .library_scroll
            .scroll_to_item(0, ScrollStrategy::Top);
    }

    fn select_entry(&mut self, entry: RailEntry, cx: &mut Context<'_, Self>) {
        match entry {
            RailEntry::Inbox => self.set_mode(DocumentsMode::Inbox, cx),
            RailEntry::Scope(scope) => {
                self.set_mode(DocumentsMode::Library, cx);
                if self.state.scope != scope {
                    self.state.scope = scope;
                    self.reset_selection();
                }
            }
        }
    }

    /// A Document Type has gone: a Library scoped to it falls back to All documents.
    pub fn type_removed(&mut self, id: DocumentTypeId, cx: &mut Context<'_, Self>) {
        if self.state.scope == LibraryScope::Type(documents::DocumentType(id)) {
            self.state.scope = LibraryScope::All;
            cx.notify();
        }
    }

    fn apply_rail_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) {
        let entries = documents::rail_entries(self.types(cx), self.today);
        let current = entries
            .iter()
            .position(|entry| *entry == self.rail_selected())
            .unwrap_or(0);
        let Some(step) = movement.step() else {
            self.state.focus = DocumentsFocus::List;
            return;
        };
        let next = lib_list_cursor::step(current, entries.len(), step);
        if let Some(entry) = entries.get(next) {
            self.select_entry(*entry, cx);
        }
    }

    fn apply_list_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) {
        let Some(step) = movement.step() else {
            self.open_selected(false, cx);
            return;
        };
        let inbox = self.state.mode == DocumentsMode::Inbox;
        let len = if inbox {
            self.inbox_rows(cx).len()
        } else {
            self.library_rows(cx).len()
        };
        if len == 0 {
            return;
        }
        let current = if inbox {
            self.inbox_at(len)
        } else {
            self.state.selected.min(len - 1)
        };
        let next = lib_list_cursor::step(current, len, step);
        if inbox {
            self.state.inbox_selected = next;
            self.state.inbox_scroll.scroll_to_item(next);
        } else {
            self.state.selected = next;
            // A non-strict scroll moves only when the row is off-screen, so a step lands the row
            // at the edge it came from, as on Transactions.
            let strategy = if next < current {
                ScrollStrategy::Top
            } else {
                ScrollStrategy::Bottom
            };
            self.state.library_scroll.scroll_to_item(next, strategy);
        }
    }

    /// Leaves the Search, keeping its query.
    fn stop_search(&mut self, cx: &mut Context<'_, Self>) {
        self.searching = false;
        cx.emit(ViewEvent::ExitMode);
    }

    /// Puts the Library on All documents, clears the search and selects `id`, so a Document just
    /// added or edited is on screen whatever scope was showing.
    pub fn select_document(&mut self, id: DocumentId, cx: &mut Context<'_, Self>) {
        let in_scope = self
            .documents(cx)
            .iter()
            .find(|document| document.id == id)
            .is_some_and(|document| self.state.scope.contains(document, self.types(cx)));
        if !in_scope {
            self.state.scope = LibraryScope::All;
        }
        self.set_mode(DocumentsMode::Library, cx);
        self.state.focus = DocumentsFocus::List;
        self.state.query.clear();
        let at = self
            .library_rows(cx)
            .iter()
            .position(|document| document.id == id)
            .unwrap_or(0);
        self.state.selected = at;
        self.state
            .library_scroll
            .scroll_to_item(at, ScrollStrategy::Center);
        cx.notify();
    }

    /// Focuses `id` in the Inbox, wherever it now sits (editing its facts clears a skip).
    pub fn focus_inbox_document(&mut self, id: DocumentId, cx: &mut Context<'_, Self>) {
        if let Some(at) = self
            .inbox_rows(cx)
            .iter()
            .position(|document| document.id == id)
        {
            self.state.inbox_selected = at;
            self.state.inbox_scroll.scroll_to_item(at);
        }
        cx.notify();
    }

    /// An Accept all that filed `batch`: remembered for `u`, with focus back at the Inbox's top.
    pub fn accepted_batch(&mut self, batch: FilingUndo, cx: &mut Context<'_, Self>) {
        self.state.undo = Some(batch);
        self.state.inbox_selected = 0;
        self.state.inbox_scroll.set_offset(gpui::Point::default());
        cx.notify();
    }

    // -----------------------------------------------------------------------------------------
    // Clicks
    // -----------------------------------------------------------------------------------------

    pub(super) fn click_rail(&mut self, entry: RailEntry, cx: &mut Context<'_, Self>) {
        self.select_entry(entry, cx);
        self.state.focus = DocumentsFocus::Index;
        cx.notify();
    }

    pub(super) fn click_row(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        if self.state.mode == DocumentsMode::Inbox {
            self.state.inbox_selected = index;
        } else {
            self.state.selected = index;
        }
        self.state.focus = DocumentsFocus::List;
        cx.notify();
    }

    pub(super) fn click_sort(&mut self, sort: LibrarySort, cx: &mut Context<'_, Self>) {
        if self.state.sort != sort {
            self.state.sort = sort;
            self.reset_selection();
        }
        cx.notify();
    }

    /// A click on the search box: the same as pressing `/`.
    pub(super) fn click_search(&mut self, cx: &mut Context<'_, Self>) {
        if self.start_search(cx) {
            cx.emit(ViewEvent::EnterMode(ViewMode::Search));
        }
    }

    /// A row's own Accept (or File…) button.
    pub(super) fn click_accept_row(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        self.state.inbox_selected = index;
        self.state.focus = DocumentsFocus::List;
        self.accept_focused(cx);
        cx.notify();
    }

    /// An Other candidate, accepted instead of the Suggested Link.
    pub(super) fn click_candidate(
        &mut self,
        transaction_id: TransactionId,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(id) = self.selected_document(cx).map(|document| document.id) {
            self.accept(id, transaction_id, cx);
        }
        cx.notify();
    }

    // -----------------------------------------------------------------------------------------
    // Files and links
    // -----------------------------------------------------------------------------------------

    fn status(&self, text: String, cx: &mut Context<'_, Self>) {
        cx.emit(ViewEvent::SetStatus(Some(text)));
    }

    fn toast(&self, kind: ToastKind, text: String, cx: &mut Context<'_, Self>) {
        cx.emit(ViewEvent::RaiseToast { kind, text });
    }

    fn open_dialog(&self, dialog: DocumentsDialog, cx: &mut Context<'_, Self>) {
        cx.emit(ViewEvent::OpenDialog(Box::new(OpenDialog::Documents(
            dialog,
        ))));
    }

    /// `enter` / **Open** (`reveal` false) and `o` / **Show in folder** (`reveal` true): hands the
    /// file to the OS when it exists, and says so when it does not.
    pub(super) fn open_selected(&mut self, reveal: bool, cx: &mut Context<'_, Self>) {
        let Some(document) = self.selected_document(cx) else {
            self.status(crate::msg::desktop_documents_status_no_document(), cx);
            return;
        };
        let path = documents::resolve_path(&document.path.to_string_lossy());
        let name = document.file_name();
        if !path.exists() {
            let shown = path.to_string_lossy().into_owned();
            self.toast(
                ToastKind::Warning,
                crate::msg::desktop_documents_status_missing(&name, &shown),
                cx,
            );
            return;
        }
        if reveal {
            cx.reveal_path(&path);
        } else {
            cx.open_with_system(&path);
        }
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.status(
            if reveal {
                crate::msg::desktop_documents_status_shown(&name)
            } else {
                crate::msg::desktop_documents_status_opened(&name)
            },
            cx,
        );
    }

    /// `l`, **+ Link to…**, **Link elsewhere…** and **File…**: opens the one link picker, to toggle
    /// a Link in the Library or to file the focused Unfiled Document from the Inbox.
    pub(super) fn link_picker(&mut self, cx: &mut Context<'_, Self>) {
        let Some(document) = self.selected_document(cx) else {
            self.status(crate::msg::desktop_documents_status_no_document(), cx);
            return;
        };
        let id = document.id;
        let state = match self.state.mode {
            DocumentsMode::Library => PickerState::new(Purpose::Link(id), document.date, None),
            DocumentsMode::Inbox => {
                let (anchor, doc_type) =
                    document
                        .intake
                        .as_ref()
                        .map_or((document.date, None), |intake| {
                            (
                                intake.facts.date.unwrap_or(intake.received_at),
                                intake.facts.doc_type,
                            )
                        });
                PickerState::new(Purpose::File(id), anchor, doc_type)
            }
        };
        self.open_picker(state, cx);
    }

    fn open_picker(&mut self, state: PickerState, cx: &mut Context<'_, Self>) {
        let types = self
            .types(cx)
            .iter()
            .map(|row| documents::DocumentType(row.id))
            .collect();
        self.open_dialog(
            DocumentsDialog::Picker(Box::new(state.with_types(types))),
            cx,
        );
    }

    /// The picker's rows for `state`: derived afresh from the stores and the Document's Links.
    pub fn picker_rows(&self, state: &PickerState, cx: &App) -> Vec<PickerRow> {
        let lookups = self.lookups(cx);
        let current: Vec<DocumentLink> =
            documents::get(self.documents(cx), state.purpose.document())
                .map(|document| document.links.clone())
                .unwrap_or_default();
        documents::picker::rows(
            state,
            &documents::picker::Sources {
                lookups: &lookups,
                current: &current,
            },
        )
    }

    /// `L`: follows a Link. In the Library one Link jumps straight there, several open a small list
    /// of them, and none is a quiet no-op. In the Inbox it follows the row's Suggested Link, so it
    /// can be checked before `y`.
    fn follow_link(&mut self, cx: &mut Context<'_, Self>) {
        let Some(document) = self.selected_document(cx) else {
            return;
        };
        let id = document.id;
        match self.state.mode {
            DocumentsMode::Library => match document.links.as_slice() {
                [] => {}
                [only] => {
                    let only = *only;
                    self.goto_link(only, cx);
                }
                _ => {
                    let state = PickerState::new(Purpose::Follow(id), document.date, None);
                    self.open_picker(state, cx);
                }
            },
            DocumentsMode::Inbox => {
                let transaction_id = self.suggestion_for(document, cx);
                if let Some(transaction_id) = transaction_id {
                    self.goto_link(DocumentLink::Transaction(transaction_id), cx);
                }
            }
        }
    }

    /// The Transaction the Document's Suggested Link points at, if it has one.
    fn suggestion_for(&self, document: &Document, cx: &App) -> Option<TransactionId> {
        documents::suggestion(
            document,
            self.documents(cx),
            self.sources.ledger.read(cx).ledger().transactions(),
            self.sources.ledger.read(cx).ledger().payees(),
        )
        .best()
        .map(|best| best.transaction_id)
    }

    /// Hands off to the record a Link points at, with it selected. There is no back-history: `g f`
    /// returns to Documents.
    pub fn goto_link(&mut self, link: DocumentLink, cx: &mut Context<'_, Self>) {
        cx.emit(match link {
            DocumentLink::Transaction(id) => ViewEvent::Reveal(Reveal::Transaction(id)),
            DocumentLink::Account(id) => ViewEvent::Reveal(Reveal::Account(id)),
            DocumentLink::Payee(id) => ViewEvent::Reveal(Reveal::Payee(id)),
            DocumentLink::BillPlan(id) => ViewEvent::Reveal(Reveal::BillPlan(id)),
            DocumentLink::InventoryItem(_) => ViewEvent::Navigate(Noun::Inventory),
        });
    }

    /// A click on a LINKED TO row's ×: removes the Link at once, with a Toast and no confirm.
    pub(super) fn unlink(&mut self, link: DocumentLink, cx: &mut Context<'_, Self>) {
        if let Some(id) = self.selected_document(cx).map(|document| document.id) {
            let name = self.link_name(link, cx);
            if self.apply(cx, ToggleDocumentLink { id, link }) == Ok(false) {
                self.toast(
                    ToastKind::Info,
                    crate::msg::desktop_documents_toast_unlinked(&name),
                    cx,
                );
            }
        }
        cx.notify();
    }

    /// The text a Link reads as in the picker, for toasts.
    fn link_name(&self, link: DocumentLink, cx: &App) -> String {
        let Some(id) = self.selected_document(cx).map(|document| document.id) else {
            return String::new();
        };
        let state = PickerState::new(Purpose::Follow(id), self.today, None);
        let lookups = self.lookups(cx);
        documents::picker::rows(
            &state,
            &documents::picker::Sources {
                lookups: &lookups,
                current: &[link],
            },
        )
        .into_iter()
        .next()
        .map(|row| row.text)
        .unwrap_or_default()
    }

    // -----------------------------------------------------------------------------------------
    // Inbox: accept, skip, undo, accept all
    // -----------------------------------------------------------------------------------------

    /// `y` and **Accept & next**: files the focused Document under its Suggested Link. A Document
    /// with no suggestion is filed by hand instead, through the picker.
    pub(super) fn accept_focused(&mut self, cx: &mut Context<'_, Self>) {
        let Some(document) = self.selected_document(cx) else {
            return;
        };
        let id = document.id;
        match self.suggestion_for(document, cx) {
            Some(transaction_id) => self.accept(id, transaction_id, cx),
            None => self.link_picker(cx),
        }
    }

    /// Files `id` under `transaction_id`, remembers it for `u` and moves focus to the next row.
    pub fn accept(
        &mut self,
        id: DocumentId,
        transaction_id: TransactionId,
        cx: &mut Context<'_, Self>,
    ) {
        let before = self.before_filing(id, cx);
        let Ok(entry) = self.apply(cx, AcceptDocument { id, transaction_id }) else {
            return;
        };
        self.after_filing(id, before, entry, cx);
    }

    /// Files `id` by hand with a Document Type and at most one Link, as one filing action.
    pub fn file_by_hand(
        &mut self,
        id: DocumentId,
        doc_type: documents::DocumentType,
        link: Option<DocumentLink>,
        cx: &mut Context<'_, Self>,
    ) {
        let before = self.before_filing(id, cx);
        let Ok(entry) = self.apply(cx, FileDocumentByHand { id, doc_type, link }) else {
            return;
        };
        self.after_filing(id, before, entry, cx);
    }

    /// Where `id` sits in the Inbox and what it is called, read before it leaves the Inbox.
    fn before_filing(&self, id: DocumentId, cx: &App) -> (Option<usize>, usize, String) {
        let rows = self.inbox_rows(cx);
        (
            rows.iter().position(|document| document.id == id),
            rows.len(),
            documents::get(self.documents(cx), id)
                .map(|document| document.title.clone())
                .unwrap_or_default(),
        )
    }

    /// What every filing does: remembers it as the one undoable action, moves focus to the next row
    /// and raises the Toast.
    fn after_filing(
        &mut self,
        id: DocumentId,
        (at, len, title): (Option<usize>, usize, String),
        entry: documents::FiledEntry,
        cx: &mut Context<'_, Self>,
    ) {
        self.state.undo = Some(FilingUndo {
            entries: vec![entry],
            focus: Some(id),
        });
        if let Some(at) = at {
            let next = documents::focus_after_filing(at, len).unwrap_or(0);
            self.state.inbox_selected = next;
            self.state.inbox_scroll.scroll_to_item(next);
        }
        self.toast(
            ToastKind::Success,
            crate::msg::desktop_documents_toast_filed(&title),
            cx,
        );
        cx.notify();
    }

    /// `x` / **Skip**: sets the focused Document aside to the bottom of the Inbox for the session.
    /// Focus stays at the same position, which is now the next row.
    pub(super) fn skip_focused(&mut self, cx: &mut Context<'_, Self>) {
        let Some(id) = self.selected_document(cx).map(|document| document.id) else {
            return;
        };
        if self.apply(cx, SkipDocument(id)).is_ok() {
            let len = self.inbox_rows(cx).len();
            self.state.inbox_selected = self.inbox_at(len);
        }
        cx.notify();
    }

    /// `u`: reverses the last filing action as a unit and puts focus back on its Document.
    fn undo_last(&mut self, cx: &mut Context<'_, Self>) {
        let Some(record) = self.state.undo.take() else {
            self.status(crate::msg::desktop_documents_status_nothing_to_undo(), cx);
            return;
        };
        let count = record.entries.len();
        let Ok(focus) = self.apply(cx, UndoFiling(record));
        self.set_mode(DocumentsMode::Inbox, cx);
        if let Some(id) = focus
            && let Some(at) = self
                .inbox_rows(cx)
                .iter()
                .position(|document| document.id == id)
        {
            self.state.inbox_selected = at;
            self.state.inbox_scroll.scroll_to_item(at);
        }
        self.toast(
            ToastKind::Info,
            crate::msg::desktop_documents_toast_undone(i64::try_from(count).unwrap_or(i64::MAX)),
            cx,
        );
    }

    /// **Watched folder…**: out of scope for now, so it says so.
    pub(super) fn watched_folder(&mut self, cx: &mut Context<'_, Self>) {
        self.toast(
            ToastKind::Info,
            crate::msg::desktop_documents_watched_folder_toast(),
            cx,
        );
    }

    // -----------------------------------------------------------------------------------------
    // Dialogs the page builds; `Shell`'s Dialog host draws and confirms them
    // -----------------------------------------------------------------------------------------

    /// `Y`, **Accept all strong matches** and `:documents accept-all`: asks first, with the count.
    pub fn open_accept_all(&mut self, cx: &mut Context<'_, Self>) {
        let count = documents::strong_count(
            self.documents(cx),
            self.sources.ledger.read(cx).ledger().transactions(),
            self.sources.ledger.read(cx).ledger().payees(),
        );
        if count == 0 {
            self.status(crate::msg::desktop_documents_status_no_strong(), cx);
            return;
        }
        self.open_dialog(DocumentsDialog::AcceptAll(count), cx);
    }

    /// `e` in the Inbox: the focused Document's Extracted Facts.
    fn open_facts(&mut self, cx: &mut Context<'_, Self>) {
        let options = self.options(cx);
        let Some(document) = self.selected_document(cx) else {
            self.status(crate::msg::desktop_documents_status_no_document(), cx);
            return;
        };
        let facts = document
            .intake
            .as_ref()
            .map(|intake| intake.facts.clone())
            .unwrap_or_default();
        let id = document.id;
        let form = FactsForm::new(&facts, &options, self.today, self.date_style);
        self.open_dialog(DocumentsDialog::Facts(id, Box::new(form)), cx);
    }

    pub fn open_add(&mut self, cx: &mut Context<'_, Self>) {
        let options = self.options(cx);
        let form = DocumentForm::new(&options, self.state.scope, self.today, self.date_style);
        self.open_dialog(DocumentsDialog::Add(Box::new(form)), cx);
    }

    pub fn open_import(&mut self, cx: &mut Context<'_, Self>) {
        self.open_dialog(
            DocumentsDialog::Import(ImportForm::default(), Vec::new()),
            cx,
        );
    }

    fn open_edit(&mut self, cx: &mut Context<'_, Self>) {
        let options = self.options(cx);
        let Some(document) = self.selected_document(cx) else {
            self.status(crate::msg::desktop_documents_status_no_document(), cx);
            return;
        };
        let id = document.id;
        let form = DocumentForm::for_edit(document, &options, self.today, self.date_style);
        self.open_dialog(DocumentsDialog::Edit(id, Box::new(form)), cx);
    }
}
