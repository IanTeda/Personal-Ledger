//! The **Document Types** Settings page (ADR-0037, and its amendment's one folder per page):
//! `SettingsPage::Documents`, the Ledger's Document Types in type-filter order. [`DocumentsPage`]
//! reads its rows from [`LedgerStore`], renders itself (`render.rs`), handles its keys and
//! movement, builds its Dialogs (`dialogs/`) and reaches `Shell` only through [`ViewEvent`];
//! `Shell`'s Dialog host draws them and confirms them through `apply.rs`. The Settings View
//! forwards to it while the page is open.
//!
//! The Documents destination (`src/documents/`, `src/view/documents/`) is not part of the page:
//! the page reads the rows it shares with the destination through `lib_documents::types`.

pub mod apply;
pub mod dialogs;
pub mod form;
pub(crate) mod render;

use gpui::{App, Context, Entity, EventEmitter, Keystroke, Subscription};
use lib_core::DocumentTypeId;
use lib_documents::types::{self as types, DocumentTypeRow};
use lib_ledger::{Domain, MoveDocumentType};

use crate::{
    chrome::{dialog_host::OpenDialog, statusline::PageStatus},
    ledger_store::{LedgerStore, notify_on},
    navigation::key_router::Movement,
    settings::hints::settings_documents_hints,
    view::event::ViewEvent,
};
use dialogs::DocumentTypesDialog;
use form::{DocumentTypeForm, RemoveForm};
pub use render::scope_text;

impl EventEmitter<ViewEvent> for DocumentsPage {}

/// Emitted when a click lands on the page, so the Settings View, which owns the page's focus,
/// moves it from the index into the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RowClicked;

impl EventEmitter<RowClicked> for DocumentsPage {}

/// The Document Types page's own state: the type id under the cursor.
pub struct DocumentsPage {
    ledger: Entity<LedgerStore>,
    /// It may name a type that no longer exists, so read it through [`Self::selected_id`].
    selected: Option<DocumentTypeId>,
    /// Whether the page, not Settings' index, has keyboard focus, told by the Settings View as it
    /// draws the page.
    pub(crate) page_focused: bool,
    /// Repaints the page when the Document Types its rows read change.
    _types_changed: Subscription,
}

impl DocumentsPage {
    pub fn new(ledger: Entity<LedgerStore>, cx: &mut Context<'_, Self>) -> Self {
        let _types_changed = notify_on(&ledger, Domain::DocumentTypes, cx);
        Self {
            ledger,
            selected: None,
            page_focused: false,
            _types_changed,
        }
    }

    pub(crate) fn types<'a>(&self, cx: &'a App) -> &'a [DocumentTypeRow] {
        self.ledger.read(cx).ledger().document_types()
    }

    pub fn set_page_focused(&mut self, focused: bool) {
        self.page_focused = focused;
    }

    // -----------------------------------------------------------------------------------------
    // The interface the Settings View forwards to while the page is open (ADR-0037)
    // -----------------------------------------------------------------------------------------

    /// `J`/`K` move the selected type a place, which is also its place in the Documents Type
    /// filter. The router would otherwise take them, so the page claims them ahead of it.
    pub fn key_ahead(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || !modifiers.shift {
            return false;
        }
        match keystroke.key.as_str() {
            "j" => self.move_selected(1, cx),
            "k" => self.move_selected(-1, cx),
            _ => return false,
        }
        true
    }

    /// The page's bare `n`, `e` and `x`, each through the same handler its button uses.
    /// Returns whether the key was one of them.
    pub fn handle_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || modifiers.shift {
            return false;
        }
        match keystroke.key.as_str() {
            "n" => self.open_add_dialog(cx),
            "e" => {
                if let Some(id) = self.selected_id(cx) {
                    self.open_edit_dialog(id, cx);
                }
            }
            "x" => {
                if let Some(id) = self.selected_id(cx) {
                    self.open_remove_dialog(id, cx);
                }
            }
            _ => return false,
        }
        true
    }

    /// `j`/`k`/`g`/`G`/`Ctrl-d`/`Ctrl-u` walk the types in the user's order.
    pub fn apply_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) {
        let Some(step) = movement.step() else {
            return;
        };
        let ids: Vec<DocumentTypeId> = self.types(cx).iter().map(|row| row.id).collect();
        self.selected = lib_list_cursor::step_id(&ids, self.selected_id(cx), step);
        cx.notify();
    }

    /// The page has nothing of its own to close: `Esc` is left to Settings, which steps back to
    /// the index.
    pub fn escape(&mut self, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// The page has no Search: `/` is inert there.
    pub fn start_search(&mut self, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    pub fn search_key(&mut self, _keystroke: &Keystroke, _cx: &mut Context<'_, Self>) -> bool {
        false
    }

    /// The page's legend and its type and file counts.
    pub fn page_status(&self, cx: &App) -> Option<PageStatus> {
        Some(PageStatus {
            hints: settings_documents_hints(),
            right: scope_text(self.types(cx)),
        })
    }

    /// The heading's meta: how many types and how many Filed files are typed.
    pub fn scope_note(&self, cx: &App) -> String {
        scope_text(self.types(cx))
    }

    // -----------------------------------------------------------------------------------------
    // Selection
    // -----------------------------------------------------------------------------------------

    /// The selected type's id: the stored id while it still exists, else the first row, so the
    /// cursor is never lost.
    pub fn selected_id(&self, cx: &App) -> Option<DocumentTypeId> {
        lib_list_cursor::resolve(self.types(cx).iter().map(|row| row.id), self.selected)
    }

    /// Selects type `id`, if it still exists.
    pub fn select_id(&mut self, id: DocumentTypeId, cx: &mut Context<'_, Self>) {
        if types::get(self.types(cx), id).is_some() {
            self.selected = Some(id);
            cx.notify();
        }
    }

    /// Moves the selected type `delta` places in the user's order, keeping it selected.
    fn move_selected(&mut self, delta: isize, cx: &mut Context<'_, Self>) {
        let Some(id) = self.selected_id(cx) else {
            return;
        };
        let command = MoveDocumentType { id, delta };
        let Ok(_moved) = self.ledger.update(cx, |store, cx| store.apply(cx, command));
        self.select_id(id, cx);
    }

    // -----------------------------------------------------------------------------------------
    // Clicks
    // -----------------------------------------------------------------------------------------

    /// A click on a row: selects it and moves focus into the page.
    pub(crate) fn click_row(&mut self, id: DocumentTypeId, cx: &mut Context<'_, Self>) {
        self.select_id(id, cx);
        cx.emit(RowClicked);
    }

    /// **+ Add document type**: the same handler `n` reaches.
    pub(crate) fn click_add(&mut self, cx: &mut Context<'_, Self>) {
        cx.emit(RowClicked);
        self.open_add_dialog(cx);
    }

    pub(crate) fn click_edit(&mut self, id: DocumentTypeId, cx: &mut Context<'_, Self>) {
        self.select_id(id, cx);
        cx.emit(RowClicked);
        self.open_edit_dialog(id, cx);
    }

    pub(crate) fn click_remove(&mut self, id: DocumentTypeId, cx: &mut Context<'_, Self>) {
        self.select_id(id, cx);
        cx.emit(RowClicked);
        self.open_remove_dialog(id, cx);
    }

    // -----------------------------------------------------------------------------------------
    // Dialogs
    // -----------------------------------------------------------------------------------------

    fn open_add_dialog(&mut self, cx: &mut Context<'_, Self>) {
        let form = DocumentTypeForm::new(self.types(cx));
        open_dialog(DocumentTypesDialog::Add(form), cx);
    }

    /// Opens the Edit dialog filled from type `id`. A no-op if the type is gone.
    fn open_edit_dialog(&mut self, id: DocumentTypeId, cx: &mut Context<'_, Self>) {
        let rows = self.types(cx);
        let Some(row) = types::get(rows, id) else {
            return;
        };
        let form = DocumentTypeForm::from_row(row, rows);
        open_dialog(DocumentTypesDialog::Edit(id, form), cx);
    }

    /// Other, the Default, has no remove action: it gets a notice instead, and the status line
    /// says so.
    fn open_remove_dialog(&mut self, id: DocumentTypeId, cx: &mut Context<'_, Self>) {
        let rows = self.types(cx);
        let Some(row) = types::get(rows, id) else {
            return;
        };
        let dialog = if row.is_default {
            cx.emit(ViewEvent::SetStatus(Some(
                crate::msg::desktop_document_types_hint_default_kept(),
            )));
            DocumentTypesDialog::DefaultNotice
        } else {
            DocumentTypesDialog::Remove(id, RemoveForm::new(rows, id))
        };
        open_dialog(dialog, cx);
    }

    // -----------------------------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------------------------

    /// The type names in order, the selected row's name, and each type's Filed file count. The
    /// open Dialog is `Shell`'s, which fills it in.
    #[doc(hidden)]
    pub fn snapshot(&self, cx: &App) -> SettingsDocumentTypesSnapshot {
        let rows = self.types(cx);
        SettingsDocumentTypesSnapshot {
            document_type_names: rows.iter().map(|row| row.name.clone()).collect(),
            document_type_ids: rows.iter().map(|row| row.id).collect(),
            selected_document_type: self
                .selected_id(cx)
                .and_then(|id| types::get(rows, id))
                .map(|row| row.name.clone()),
            document_type_files: rows.iter().map(|row| row.files).collect(),
            document_type_dialog: None,
        }
    }
}

/// The Document Types page: names in order, the selected row's name, each type's Filed file count,
/// and the open dialog's kind (`add`, `edit`, `remove` or `notice`), which `Shell` fills in.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsDocumentTypesSnapshot {
    pub document_type_names: Vec<String>,
    /// Each type's id, in the same order as the names.
    pub document_type_ids: Vec<DocumentTypeId>,
    pub selected_document_type: Option<String>,
    pub document_type_files: Vec<u32>,
    pub document_type_dialog: Option<String>,
}

impl DocumentTypesDialog {
    /// The kind of dialog, for the Settings snapshot.
    #[doc(hidden)]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Add(_) => "add",
            Self::Edit(..) => "edit",
            Self::Remove(..) => "remove",
            Self::DefaultNotice => "notice",
        }
    }
}

/// Asks `Shell`'s Dialog host to open a Document types dialog.
fn open_dialog(dialog: DocumentTypesDialog, cx: &mut Context<'_, DocumentsPage>) {
    cx.emit(ViewEvent::OpenDialog(Box::new(OpenDialog::DocumentTypes(
        dialog,
    ))));
}
