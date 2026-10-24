//! The **Document Types** Settings page (ADR-0037, and its amendment's one folder per page):
//! `SettingsPage::Documents`, the Ledger's Document Types in type-filter order. [`DocumentsPage`]
//! reads its rows from [`LedgerStore`], renders itself (`render.rs`), handles its keys and
//! movement, builds its Dialogs (`dialogs/`) and reaches `Root` only through [`ViewEvent`];
//! `Root`'s Dialog host draws them and confirms them through `apply.rs`. The Settings View
//! forwards to it while the page is open.
//!
//! The Documents destination (`src/documents/`, `src/view/documents/`) is not part of the page:
//! the page reads the rows it shares with the destination through `lib_documents::types`.

pub mod apply;
pub mod dialogs;
pub mod form;
pub(crate) mod render;

use crate::settings::SettingsPage;
use crate::settings::{
    page::PageRowClicked,
    pages::{Page, focus_on_row_click},
    state::SettingsView,
};
use gpui::{AnyElement, IntoElement as _};
use gpui::{App, Context, Entity, EventEmitter, Keystroke, Subscription};
use lib_core::DocumentTypeId;
use lib_documents::types::{self as types, DocumentTypeRow};
use lib_ledger::{Domain, MoveDocumentType};

use crate::{
    ledger_store::{LedgerStore, notify_on},
    navigation::movement::Movement,
    settings::hints::settings_documents_hints,
    shell::view::{
        dialog_host::open_dialog,
        statusline::{Hint, PageStatus, owned_hints},
    },
    view::event::ViewEvent,
};
use apply::DocumentTypeOutcome;
use dialogs::DocumentTypesDialog;
use form::{DocumentTypeForm, RemoveForm};
pub use render::scope_text;

impl EventEmitter<ViewEvent> for DocumentsPage {}

impl EventEmitter<PageRowClicked> for DocumentsPage {}

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

    /// The heading's meta: how many types and how many Filed files are typed.
    pub(crate) fn scope_note(&self, cx: &App) -> String {
        scope_text(self.types(cx))
    }
}

impl Page for DocumentsPage {
    const PAGE: SettingsPage = SettingsPage::Documents;

    /// `J`/`K` move the selected type a place, which is also its place in the Documents Type
    /// filter. The router would otherwise take them, so the page claims them ahead of it.
    fn key_ahead(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
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
    fn handle_key(&mut self, keystroke: &Keystroke, cx: &mut Context<'_, Self>) -> bool {
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
    fn apply_movement(&mut self, movement: Movement, cx: &mut Context<'_, Self>) -> bool {
        let Some(step) = movement.step() else {
            return true;
        };
        let ids: Vec<DocumentTypeId> = self.types(cx).iter().map(|row| row.id).collect();
        self.selected = lib_list_cursor::step_id(&ids, self.selected_id(cx), step);
        cx.notify();
        true
    }

    /// The page's legend and its type and file counts.
    fn page_status(&self, cx: &App) -> Option<PageStatus> {
        Some(PageStatus {
            hints: owned_hints(settings_documents_hints()),
            right: scope_text(self.types(cx)),
        })
    }

    fn set_page_focused(&mut self, focused: bool) {
        self.page_focused = focused;
    }

    fn hints(&self, _cx: &App) -> Vec<Hint> {
        owned_hints(settings_documents_hints())
    }

    fn heading_meta(&self, cx: &App) -> AnyElement {
        self.scope_note(cx).into_any_element()
    }

    fn subscribe_row_clicks(
        entity: &Entity<Self>,
        cx: &mut Context<'_, SettingsView>,
    ) -> Option<Subscription> {
        focus_on_row_click(entity, cx)
    }
}

impl DocumentsPage {
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
        cx.emit(PageRowClicked);
    }

    /// **+ Add document type**: the same handler `n` reaches.
    pub(crate) fn click_add(&mut self, cx: &mut Context<'_, Self>) {
        cx.emit(PageRowClicked);
        self.open_add_dialog(cx);
    }

    pub(crate) fn click_edit(&mut self, id: DocumentTypeId, cx: &mut Context<'_, Self>) {
        self.select_id(id, cx);
        cx.emit(PageRowClicked);
        self.open_edit_dialog(id, cx);
    }

    pub(crate) fn click_remove(&mut self, id: DocumentTypeId, cx: &mut Context<'_, Self>) {
        self.select_id(id, cx);
        cx.emit(PageRowClicked);
        self.open_remove_dialog(id, cx);
    }

    // -----------------------------------------------------------------------------------------
    // Dialogs
    // -----------------------------------------------------------------------------------------

    /// Lands a confirmed Dialog: a save or a removal selects the type the Dialog named.
    fn land(&mut self, outcome: DocumentTypeOutcome, cx: &mut Context<'_, Self>) {
        match outcome {
            DocumentTypeOutcome::Saved(id) => self.select_id(id, cx),
            DocumentTypeOutcome::Removed { selected } => {
                if let Some(id) = selected {
                    self.select_id(id, cx);
                }
            }
            DocumentTypeOutcome::Dismissed => {}
        }
    }

    fn open_add_dialog(&mut self, cx: &mut Context<'_, Self>) {
        let form = DocumentTypeForm::new(self.types(cx));
        open_dialog(DocumentTypesDialog::Add(form), Self::land, cx);
    }

    /// Opens the Edit dialog filled from type `id`. A no-op if the type is gone.
    fn open_edit_dialog(&mut self, id: DocumentTypeId, cx: &mut Context<'_, Self>) {
        let rows = self.types(cx);
        let Some(row) = types::get(rows, id) else {
            return;
        };
        let form = DocumentTypeForm::from_row(row, rows);
        open_dialog(DocumentTypesDialog::Edit(id, form), Self::land, cx);
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
        open_dialog(dialog, Self::land, cx);
    }

    // -----------------------------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------------------------

    /// The type names in order, the selected row's name, and each type's Filed file count.
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
        }
    }
}

/// The Document Types page: names in order, the selected row's name and each type's Filed file
/// count. The open Dialog is read back through the Dialog host, not here.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsDocumentTypesSnapshot {
    pub document_type_names: Vec<String>,
    /// Each type's id, in the same order as the names.
    pub document_type_ids: Vec<DocumentTypeId>,
    pub selected_document_type: Option<String>,
    pub document_type_files: Vec<u32>,
}
