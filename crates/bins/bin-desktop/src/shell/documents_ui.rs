//! `Shell`'s side of the Documents destination: the Library's scope, search, sort and selection,
//! the index-rail and list focus zones, the Add, Import and Edit dialogs, opening a file in the OS
//! and the page's status legend. A child of `shell` so it reads `Shell`'s private fields, without
//! growing that file further. The Documents and Types live in `DocumentsStore` and the page's
//! state in `DocumentsView` (ADR-0032); the helpers below read and edit them through `cx`. The rules themselves are `documents` and `documents::form`; this only
//! wires them to keys and clicks.

use std::path::Path;
use std::rc::Rc;

use gpui::{Context, Keystroke, Window};
use lib_toast::ToastKind;

use super::{OpenDialog, Shell, key_dispatch::typed_char};
use crate::{
    chrome::statusline::PageStatus,
    documents::form::{
        DocumentField, DocumentForm, DocumentOptions, DocumentsDialog, FactsField, FactsForm,
        ImportForm, ImportOutcome,
    },
    documents::model::{self as view_model, Lookups},
    documents::picker::{PickerRequest, PickerRow, PickerState, Purpose},
    documents::types::DocumentTypeRow,
    documents::{
        self, Document, DocumentLink, DocumentsMode, KeyDateKind, LibraryScope, LibrarySort,
        RailEntry,
    },
    navigation::key_router::Movement,
    navigation::nav::{FocusZone, InputMode, Noun},
    settings::SettingsSection,
    view::documents::{
        self as documents_view, DocumentsFocus, DocumentsPageProps, dialogs,
        state::{DocumentsData, DocumentsEvent, DocumentsState},
    },
};

/// `ctrl-d` / `ctrl-u` on the Library list: rows per half page.
const DOCUMENTS_HALF_PAGE: usize = 5;

fn key_label_for_options(kind: Option<KeyDateKind>) -> String {
    match kind {
        None => crate::msg::desktop_documents_key_none(),
        Some(KeyDateKind::Renews) => crate::msg::desktop_documents_detail_key_renews(),
        Some(KeyDateKind::Ends) => crate::msg::desktop_documents_detail_key_ends(),
        Some(KeyDateKind::Expires) => crate::msg::desktop_documents_detail_key_expires(),
        Some(KeyDateKind::Revalue) => crate::msg::desktop_documents_detail_key_revalue(),
    }
}

/// The Library list's status-line legend: the handoff's own strings.
fn library_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("enter", crate::msg::desktop_hint_open()),
        ("/", crate::msg::desktop_hint_search()),
        ("l", crate::msg::desktop_hint_link()),
        ("L", crate::msg::desktop_hint_follow()),
        ("o", crate::msg::desktop_hint_show_in_folder()),
        ("i", crate::msg::desktop_hint_inbox()),
    ]
}

fn inbox_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_row()),
        ("y", crate::msg::desktop_hint_accept()),
        ("e", crate::msg::desktop_hint_edit()),
        ("x", crate::msg::desktop_hint_skip()),
        ("l", crate::msg::desktop_hint_link_elsewhere()),
        ("L", crate::msg::desktop_hint_follow()),
    ]
}

fn index_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_scope()),
        ("l/enter", crate::msg::desktop_hint_list()),
        ("i", crate::msg::desktop_hint_inbox()),
    ]
}

fn picker_hints() -> Vec<(&'static str, String)> {
    vec![
        ("esc", crate::msg::desktop_hint_cancel()),
        ("tab", crate::msg::desktop_hint_kind()),
        ("enter", crate::msg::desktop_hint_confirm()),
    ]
}

fn dialog_hints(import: bool) -> Vec<(&'static str, String)> {
    let mut hints = vec![
        ("esc", crate::msg::desktop_hint_cancel()),
        ("tab", crate::msg::desktop_hint_next_field()),
    ];
    hints.push((
        if import { "ctrl+enter" } else { "enter" },
        crate::msg::desktop_hint_confirm(),
    ));
    hints
}

/// Whether the path exists on disk, passed to the form checks so they stay testable.
fn exists_on_disk(path: &Path) -> bool {
    path.exists()
}

impl Shell {
    /// The Documents, read from their store Entity.
    pub(super) fn documents<'a>(&self, cx: &'a gpui::App) -> &'a [Document] {
        self.documents_store.read(cx).documents()
    }

    /// The Document Types in the user's order, read from the store Entity.
    pub(super) fn document_types<'a>(&self, cx: &'a gpui::App) -> &'a [DocumentTypeRow] {
        self.documents_store.read(cx).types()
    }

    /// Applies `change` to the shared Documents and Types, notifying every reader.
    pub(super) fn mutate_documents<R>(
        &self,
        cx: &mut gpui::App,
        change: impl FnOnce(&mut DocumentsData) -> R,
    ) -> R {
        self.documents_store
            .update(cx, |store, cx| store.mutate(cx, change))
    }

    /// The Documents page's state, read from its view Entity.
    pub(super) fn documents_state<'a>(&self, cx: &'a gpui::App) -> &'a DocumentsState {
        self.documents_view.read(cx).state()
    }

    /// Edits the Documents page's state.
    pub(super) fn edit_documents_state<R>(
        &self,
        cx: &mut gpui::App,
        change: impl FnOnce(&mut DocumentsState) -> R,
    ) -> R {
        self.documents_view
            .update(cx, |view, cx| view.edit(cx, change))
    }

    /// Carries out what the Documents page asked for: the OS file hand-off, which needs an `App`.
    pub(super) fn handle_documents_event(
        &mut self,
        event: &DocumentsEvent,
        cx: &mut Context<'_, Self>,
    ) {
        match event {
            DocumentsEvent::OpenFile { path, reveal } => {
                if *reveal {
                    cx.reveal_path(path);
                } else {
                    cx.open_with_system(path);
                }
                let name = path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                self.chrome.status_message = Some(if *reveal {
                    crate::msg::desktop_documents_status_shown(&name)
                } else {
                    crate::msg::desktop_documents_status_opened(&name)
                });
            }
        }
        cx.notify();
    }

    pub(super) fn documents_lookups<'a>(&'a self, cx: &'a gpui::App) -> Lookups<'a> {
        Lookups {
            accounts: self.accounts.read(cx).accounts(),
            payees: &self.payees,
            plans: self.bill_plans(cx),
            types: self.document_types(cx),
            inventory: &self.inventory,
            transactions: self.transactions(cx),
            today: self.today,
            date_style: self.settings_date_style,
        }
    }

    fn documents_options(&self, cx: &gpui::App) -> DocumentOptions {
        DocumentOptions::new(self.document_types(cx), key_label_for_options)
    }

    /// The Library's rows under the current scope, search and sort.
    pub(super) fn documents_library_rows<'a>(&self, cx: &'a gpui::App) -> Vec<&'a Document> {
        documents::library_rows(
            self.document_types(cx),
            self.documents(cx),
            self.documents_state(cx).scope,
            &self.documents_state(cx).query,
            self.documents_state(cx).sort,
            self.today,
        )
    }

    fn documents_rail_selected(&self, cx: &gpui::App) -> RailEntry {
        match self.documents_state(cx).mode {
            DocumentsMode::Inbox => RailEntry::Inbox,
            DocumentsMode::Library => RailEntry::Scope(self.documents_state(cx).scope),
        }
    }

    /// The Inbox's rows: newest received first, Skipped Documents last.
    pub(super) fn documents_inbox_rows<'a>(&self, cx: &'a gpui::App) -> Vec<&'a Document> {
        documents::inbox_rows(self.documents(cx))
    }

    /// The Inbox row focus is on, a position clamped to the rows.
    fn documents_inbox_at(&self, len: usize, cx: &gpui::App) -> usize {
        self.documents_state(cx)
            .inbox_selected
            .min(len.saturating_sub(1))
    }

    /// The Document the selection is on: the Library's row or the Inbox's focused row.
    pub(super) fn documents_selected_document<'a>(
        &self,
        cx: &'a gpui::App,
    ) -> Option<&'a Document> {
        match self.documents_state(cx).mode {
            DocumentsMode::Library => {
                let rows = self.documents_library_rows(cx);
                let at = self
                    .documents_state(cx)
                    .selected
                    .min(rows.len().saturating_sub(1));
                rows.get(at).copied()
            }
            DocumentsMode::Inbox => {
                let rows = self.documents_inbox_rows(cx);
                let at = self.documents_inbox_at(rows.len(), cx);
                rows.get(at).copied()
            }
        }
    }

    /// Switches Inbox / Library, putting the list back at its top when the mode changes.
    fn set_documents_mode(&mut self, mode: DocumentsMode, cx: &mut gpui::App) {
        if self.documents_state(cx).mode != mode {
            self.edit_documents_state(cx, |state| state.mode = mode);
            self.documents_state(cx)
                .scroll
                .set_offset(gpui::Point::default());
        }
    }

    fn documents_reset_selection(&mut self, cx: &mut gpui::App) {
        self.edit_documents_state(cx, |state| state.selected = 0);
        self.documents_state(cx)
            .scroll
            .set_offset(gpui::Point::default());
    }

    pub(super) fn set_documents_persisted(
        &self,
        mode: DocumentsMode,
        scope: LibraryScope,
        sort: LibrarySort,
        cx: &mut gpui::App,
    ) {
        self.edit_documents_state(cx, |state| {
            state.mode = mode;
            state.scope = scope;
            state.sort = sort;
        });
    }

    pub fn documents_persisted(
        &self,
        cx: &gpui::App,
    ) -> (DocumentsMode, LibraryScope, LibrarySort) {
        (
            self.documents_state(cx).mode,
            self.documents_state(cx).scope,
            self.documents_state(cx).sort,
        )
    }

    /// Builds the page's props, only while Documents is the noun on show.
    pub(super) fn documents_page_props(
        &self,
        entity: &gpui::Entity<Shell>,
        cx: &gpui::App,
    ) -> DocumentsPageProps {
        let lookups = self.documents_lookups(cx);
        let rows = self.documents_library_rows(cx);
        let selected = self
            .documents_state(cx)
            .selected
            .min(rows.len().saturating_sub(1));
        let total = documents::scope_count(
            self.document_types(cx),
            self.documents(cx),
            LibraryScope::All,
        );
        let count_line = crate::msg::desktop_documents_count_line(
            &rows.len().to_string(),
            &total.to_string(),
            &documents::need_review_count(self.document_types(cx), &rows, self.today).to_string(),
        );
        let detail = rows
            .get(selected)
            .filter(|_| self.documents_state(cx).mode == DocumentsMode::Library)
            .map(|document| view_model::detail(document, &lookups));
        let row_views: Vec<_> = rows
            .iter()
            .map(|document| view_model::row(document, &lookups))
            .collect();
        let on_rail_click: documents_view::OnRailClick = {
            let entity = entity.clone();
            Rc::new(move |entry, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_documents_rail_click(entry, cx));
            })
        };
        let on_row_click: documents_view::OnRowClick = {
            let entity = entity.clone();
            Rc::new(move |index, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_documents_row_click(index, cx));
            })
        };
        let on_sort_click: documents_view::OnSortClick = {
            let entity = entity.clone();
            Rc::new(move |sort, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_documents_sort_click(sort, cx));
            })
        };
        let plain = |act: fn(&mut Shell, &mut Context<'_, Shell>)| -> documents_view::OnPlainClick {
            let entity = entity.clone();
            Rc::new(move |_window, cx| {
                entity.update(cx, act);
            })
        };
        let inbox = self.documents_inbox_rows(cx);
        let inbox_selected = self.documents_inbox_at(inbox.len(), cx);
        let inbox_views: Vec<_> = inbox
            .iter()
            .map(|document| view_model::inbox_row(document, self.documents(cx), &lookups))
            .collect();
        let inbox_detail = inbox
            .get(inbox_selected)
            .filter(|_| self.documents_state(cx).mode == DocumentsMode::Inbox)
            .map(|document| view_model::inbox_detail(document, self.documents(cx), &lookups));
        let on_accept_row_click: documents_view::OnRowClick = {
            let entity = entity.clone();
            Rc::new(move |index, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.edit_documents_state(cx, |state| {
                        state.inbox_selected = index;
                        state.focus = DocumentsFocus::List;
                    });
                    shell.documents_accept_focused(cx);
                    cx.notify();
                });
            })
        };
        let on_accept_candidate_click: documents_view::OnCandidateClick = {
            let entity = entity.clone();
            Rc::new(move |transaction_id, _window, cx| {
                entity.update(cx, |shell, cx| {
                    if let Some(id) = shell.documents_selected_document(cx).map(|d| d.id) {
                        shell.documents_accept(id, transaction_id, cx);
                    }
                    cx.notify();
                });
            })
        };
        let on_link_click: documents_view::OnLinkClick = {
            let entity = entity.clone();
            Rc::new(move |link, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_documents_link_click(link, cx));
            })
        };
        let on_unlink_click: documents_view::OnLinkClick = {
            let entity = entity.clone();
            Rc::new(move |link, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.handle_documents_unlink_click(link, cx)
                });
            })
        };
        DocumentsPageProps {
            mode: self.documents_state(cx).mode,
            focus: self.documents_state(cx).focus,
            rail: view_model::rail_rows(self.document_types(cx), self.documents(cx), self.today),
            rail_selected: self.documents_rail_selected(cx),
            rail_footer: self.documents_stored_text(cx),
            subline: match self.documents_state(cx).mode {
                DocumentsMode::Library => {
                    crate::msg::desktop_documents_subline(i64::try_from(total).unwrap_or(i64::MAX))
                }
                DocumentsMode::Inbox => crate::msg::desktop_documents_inbox_subline(
                    i64::try_from(documents::inbox_count(self.documents(cx))).unwrap_or(i64::MAX),
                ),
            },
            query: self.documents_state(cx).query.clone(),
            searching: self.nav.mode() == InputMode::Search,
            sort: self.documents_state(cx).sort,
            count_line,
            rows: row_views,
            selected,
            detail,
            inbox_rows: inbox_views,
            inbox_selected,
            inbox_detail,
            inbox_strong: documents::strong_count(
                self.documents(cx),
                self.transactions(cx),
                &self.payees,
            ),
            inbox_hints: inbox_hints(),
            list_focused: false,
            scroll: self.documents_state(cx).scroll.clone(),
            on_rail_click,
            on_row_click,
            on_sort_click,
            on_search_click: plain(|shell, cx| {
                shell.documents_start_search(cx);
                cx.notify();
            }),
            on_add_click: plain(|shell, cx| {
                shell.open_documents_add(cx);
                cx.notify();
            }),
            on_import_click: plain(|shell, cx| {
                shell.open_documents_import();
                cx.notify();
            }),
            on_open_click: plain(|shell, cx| {
                shell.documents_open_selected(false, cx);
                cx.notify();
            }),
            on_show_click: plain(|shell, cx| {
                shell.documents_open_selected(true, cx);
                cx.notify();
            }),
            on_link_click,
            on_unlink_click,
            on_add_link_click: plain(|shell, cx| {
                shell.documents_link_picker(cx);
                cx.notify();
            }),
            on_watched_click: plain(|shell, cx| {
                shell.documents_watched_folder();
                cx.notify();
            }),
            on_accept_all_click: plain(|shell, cx| {
                shell.open_documents_accept_all(cx);
                cx.notify();
            }),
            on_accept_row_click,
            on_accept_next_click: plain(|shell, cx| {
                shell.documents_accept_focused(cx);
                cx.notify();
            }),
            on_accept_candidate_click,
            on_link_elsewhere_click: plain(|shell, cx| {
                shell.documents_link_picker(cx);
                cx.notify();
            }),
            on_skip_click: plain(|shell, cx| {
                shell.documents_skip_focused(cx);
                cx.notify();
            }),
        }
    }

    /// "1.8 GB · stored beside household.pldb": the summed stub file sizes and the Ledger file.
    fn documents_stored_text(&self, cx: &gpui::App) -> String {
        crate::msg::desktop_documents_status_stored(
            &view_model::size_text(documents::total_bytes(self.documents(cx))),
            crate::chrome::statusline::STUB_LEDGER_FILE,
        )
    }

    /// The Documents page's status line for the focus zone and mode, or `None` off the page.
    pub(super) fn documents_page_status(&self, cx: &gpui::App) -> Option<PageStatus> {
        if self.nav.noun() != Noun::Documents {
            return None;
        }
        let hints = match self.documents_dialog() {
            Some(DocumentsDialog::Import(..)) => dialog_hints(true),
            Some(DocumentsDialog::Picker(_)) => picker_hints(),
            Some(_) => dialog_hints(false),
            None if self.documents_state(cx).focus == DocumentsFocus::Index => index_hints(),
            None if self.documents_state(cx).mode == DocumentsMode::Inbox => inbox_hints(),
            None => library_hints(),
        };
        Some(PageStatus {
            hints,
            right: self.documents_stored_text(cx),
        })
    }

    /// The `?` cheat-sheet's Documents group as `(action, keys)`: the index keys, then the current
    /// mode's. Empty off the page.
    pub(super) fn documents_cheat_sheet(&self, cx: &gpui::App) -> Vec<(String, &'static str)> {
        if self.nav.noun() != Noun::Documents {
            return Vec::new();
        }
        let mode = match self.documents_state(cx).mode {
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

    fn select_documents_entry(&mut self, entry: RailEntry, cx: &mut gpui::App) {
        match entry {
            RailEntry::Inbox => self.set_documents_mode(DocumentsMode::Inbox, cx),
            RailEntry::Scope(scope) => {
                self.set_documents_mode(DocumentsMode::Library, cx);
                if self.documents_state(cx).scope != scope {
                    self.edit_documents_state(cx, |state| state.scope = scope);
                    self.documents_reset_selection(cx);
                }
            }
        }
    }

    fn handle_documents_rail_click(&mut self, entry: RailEntry, cx: &mut Context<'_, Self>) {
        self.select_documents_entry(entry, cx);
        self.edit_documents_state(cx, |state| state.focus = DocumentsFocus::Index);
        cx.notify();
    }

    fn handle_documents_row_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        if self.documents_state(cx).mode == DocumentsMode::Inbox {
            self.edit_documents_state(cx, |state| state.inbox_selected = index);
        } else {
            self.edit_documents_state(cx, |state| state.selected = index);
        }
        self.edit_documents_state(cx, |state| state.focus = DocumentsFocus::List);
        cx.notify();
    }

    fn handle_documents_sort_click(&mut self, sort: LibrarySort, cx: &mut Context<'_, Self>) {
        if self.documents_state(cx).sort != sort {
            self.edit_documents_state(cx, |state| state.sort = sort);
            self.documents_reset_selection(cx);
        }
        cx.notify();
    }

    /// The Library's `/`: focuses the toolbar search from either zone. Inert in the Inbox, which
    /// draws no search box.
    pub(super) fn documents_start_search(&mut self, cx: &mut gpui::App) {
        if self.documents_state(cx).mode != DocumentsMode::Library {
            return;
        }
        self.edit_documents_state(cx, |state| state.focus = DocumentsFocus::List);
        self.nav.enter_mode(InputMode::Search);
    }

    /// `i`: toggles Inbox and Library, keeping focus in its zone.
    fn toggle_documents_mode(&mut self, cx: &mut gpui::App) {
        self.set_documents_mode(
            match self.documents_state(cx).mode {
                DocumentsMode::Library => DocumentsMode::Inbox,
                DocumentsMode::Inbox => DocumentsMode::Library,
            },
            cx,
        );
    }

    /// `j`/`k`/`gg`/`G`/`ctrl-d`/`ctrl-u`/`enter` on the Documents page, by focus zone.
    pub(super) fn apply_documents_movement(&mut self, movement: Movement, cx: &mut gpui::App) {
        match self.documents_state(cx).focus {
            DocumentsFocus::Index => self.apply_documents_rail_movement(movement, cx),
            DocumentsFocus::List => self.apply_documents_list_movement(movement, cx),
        }
    }

    fn apply_documents_rail_movement(&mut self, movement: Movement, cx: &mut gpui::App) {
        let entries = documents::rail_entries(self.document_types(cx), self.today);
        let current = entries
            .iter()
            .position(|entry| *entry == self.documents_rail_selected(cx))
            .unwrap_or(0);
        let last = entries.len().saturating_sub(1);
        let next = match movement {
            Movement::Next => (current + 1).min(last),
            Movement::Prev => current.saturating_sub(1),
            Movement::First => 0,
            Movement::Last => last,
            Movement::HalfPageDown => (current + DOCUMENTS_HALF_PAGE).min(last),
            Movement::HalfPageUp => current.saturating_sub(DOCUMENTS_HALF_PAGE),
            Movement::Enter => {
                self.edit_documents_state(cx, |state| state.focus = DocumentsFocus::List);
                return;
            }
        };
        if let Some(entry) = entries.get(next) {
            self.select_documents_entry(*entry, cx);
        }
    }

    fn apply_documents_list_movement(&mut self, movement: Movement, cx: &mut gpui::App) {
        if movement == Movement::Enter {
            self.documents_open_selected(false, cx);
            return;
        }
        let inbox = self.documents_state(cx).mode == DocumentsMode::Inbox;
        let len = if inbox {
            self.documents_inbox_rows(cx).len()
        } else {
            self.documents_library_rows(cx).len()
        };
        if len == 0 {
            return;
        }
        let current = if inbox {
            self.documents_inbox_at(len, cx)
        } else {
            self.documents_state(cx).selected.min(len - 1)
        };
        let last = len - 1;
        let next = match movement {
            Movement::Next => (current + 1).min(last),
            Movement::Prev => current.saturating_sub(1),
            Movement::First => 0,
            Movement::Last => last,
            Movement::HalfPageDown => (current + DOCUMENTS_HALF_PAGE).min(last),
            Movement::HalfPageUp => current.saturating_sub(DOCUMENTS_HALF_PAGE),
            Movement::Enter => current,
        };
        if inbox {
            self.edit_documents_state(cx, |state| state.inbox_selected = next);
        } else {
            self.edit_documents_state(cx, |state| state.selected = next);
        }
        self.documents_state(cx).scroll.scroll_to_item(next);
    }

    // -----------------------------------------------------------------------------------------
    // Keys
    // -----------------------------------------------------------------------------------------

    /// The Documents keys that sit ahead of the global router (`a` would otherwise enter Insert
    /// mode), in `Normal` mode with the view focused. `false` for any key it does not own.
    pub(super) fn handle_documents_key(
        &mut self,
        keystroke: &Keystroke,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        if self.nav.noun() != Noun::Documents
            || self.nav.focus() != FocusZone::View
            || self.nav.mode() != InputMode::Normal
        {
            return false;
        }
        let modifiers = &keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
            return false;
        }
        let library = self.documents_state(cx).mode == DocumentsMode::Library;
        let in_list = self.documents_state(cx).focus == DocumentsFocus::List;
        match keystroke.key.as_str() {
            "escape" => {
                // The innermost thing first: a committed query, then the list back to the index.
                if !self.documents_state(cx).query.is_empty() {
                    self.edit_documents_state(cx, |state| state.query.clear());
                    self.documents_reset_selection(cx);
                    true
                } else if in_list {
                    self.edit_documents_state(cx, |state| state.focus = DocumentsFocus::Index);
                    true
                } else {
                    false
                }
            }
            "i" if modifiers.shift => {
                self.open_documents_import();
                true
            }
            "i" => {
                self.toggle_documents_mode(cx);
                true
            }
            // Library only; swallowed in the Inbox so `a` does not enter Insert mode there.
            "a" if !modifiers.shift => {
                if library {
                    self.open_documents_add(cx);
                }
                true
            }
            "s" if library && !modifiers.shift => {
                self.edit_documents_state(cx, |state| state.sort = state.sort.toggle());
                self.documents_reset_selection(cx);
                true
            }
            "e" if !modifiers.shift => {
                if library {
                    self.open_documents_edit(cx);
                } else {
                    self.open_documents_facts(cx);
                }
                true
            }
            "l" | "right" if !in_list => {
                self.edit_documents_state(cx, |state| state.focus = DocumentsFocus::List);
                true
            }
            "l" if in_list && !modifiers.shift => {
                self.documents_link_picker(cx);
                true
            }
            "l" if in_list && modifiers.shift => {
                self.documents_follow_link(cx);
                true
            }
            "h" | "left" if in_list => {
                self.edit_documents_state(cx, |state| state.focus = DocumentsFocus::Index);
                true
            }
            "o" if in_list => {
                self.documents_open_selected(true, cx);
                true
            }
            // Inbox only; in the Library these fall through to the global router.
            "y" if !library && modifiers.shift => {
                self.open_documents_accept_all(cx);
                true
            }
            "y" if !library && in_list => {
                self.documents_accept_focused(cx);
                true
            }
            "x" if !library && in_list && !modifiers.shift => {
                self.documents_skip_focused(cx);
                true
            }
            "u" if !library && !modifiers.shift => {
                self.documents_undo_last(cx);
                true
            }
            _ => false,
        }
    }

    /// Keys while `InputMode::Search` is on the Documents page: typing narrows live, `Backspace`
    /// edits, `Enter` keeps the query and returns to the list. `Esc` is handled with the other
    /// modes' exit, which clears it.
    pub(super) fn handle_documents_search_key(
        &mut self,
        keystroke: &Keystroke,
        cx: &mut gpui::App,
    ) -> bool {
        match keystroke.key.as_str() {
            "enter" => {
                self.nav.exit_mode();
                true
            }
            "backspace" => {
                self.edit_documents_state(cx, |state| state.query.pop());
                self.documents_reset_selection(cx);
                true
            }
            _ => {
                let modifiers = &keystroke.modifiers;
                if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                    return false;
                }
                match typed_char(keystroke) {
                    Some(ch) if !ch.is_control() => {
                        self.edit_documents_state(cx, |state| state.query.push(ch));
                        self.documents_reset_selection(cx);
                        true
                    }
                    _ => false,
                }
            }
        }
    }

    /// `esc` out of Search: leaves the mode and clears the query.
    pub(super) fn documents_cancel_search(&mut self, cx: &mut gpui::App) {
        self.edit_documents_state(cx, |state| state.query.clear());
        self.documents_reset_selection(cx);
    }

    // -----------------------------------------------------------------------------------------
    // Files and links
    // -----------------------------------------------------------------------------------------

    /// `enter` / **Open** (`reveal` false) and `o` / **Show in folder** (`reveal` true): hands the
    /// file to the OS when it exists, and says so when it does not.
    pub(super) fn documents_open_selected(&mut self, reveal: bool, cx: &mut gpui::App) {
        let Some(document) = self.documents_selected_document(cx) else {
            self.chrome.status_message = Some(crate::msg::desktop_documents_status_no_document());
            return;
        };
        let path = documents::resolve_path(&document.path.to_string_lossy());
        let name = document.file_name();
        if !path.exists() {
            let shown = path.to_string_lossy().into_owned();
            self.raise_toast(
                ToastKind::Warning,
                crate::msg::desktop_documents_status_missing(&name, &shown),
            );
            return;
        }
        self.documents_view
            .update(cx, |view, cx| view.request_open(cx, path, reveal));
    }

    /// `l`, **+ Link to…**, **Link elsewhere…** and **File…**: opens the one link picker, to toggle
    /// a Link in the Library or to file the focused Unfiled Document from the Inbox.
    pub(super) fn documents_link_picker(&mut self, cx: &gpui::App) {
        let Some(document) = self.documents_selected_document(cx) else {
            self.chrome.status_message = Some(crate::msg::desktop_documents_status_no_document());
            return;
        };
        let id = document.id;
        let state = match self.documents_state(cx).mode {
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
        self.open_documents_picker(state, cx);
    }

    fn open_documents_picker(&mut self, state: PickerState, cx: &gpui::App) {
        let types = self
            .document_types(cx)
            .iter()
            .map(|row| documents::DocumentType(row.id))
            .collect();
        self.open_documents_dialog(DocumentsDialog::Picker(Box::new(state.with_types(types))));
    }

    /// The picker's rows for `state`: derived afresh from the stubs and the Document's Links.
    pub(super) fn documents_picker_rows(
        &self,
        state: &PickerState,
        cx: &gpui::App,
    ) -> Vec<PickerRow> {
        let lookups = self.documents_lookups(cx);
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

    /// What `enter` or a click on picker row `index` does: toggles a Link, files the Unfiled
    /// Document, or follows a Link, according to why the picker opened.
    pub(super) fn documents_picker_pick(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let Some(DocumentsDialog::Picker(state)) = self.documents_dialog().cloned() else {
            return;
        };
        let Some(row) = self.documents_picker_rows(&state, cx).get(index).cloned() else {
            return;
        };
        match state.purpose {
            Purpose::Link(id) => {
                let Some(link) = row.link else {
                    return;
                };
                let Some(added) = self.mutate_documents(cx, |data| {
                    documents::toggle_link(&mut data.documents, id, link)
                }) else {
                    return;
                };
                self.close_documents_dialog();
                self.raise_toast(
                    ToastKind::Info,
                    if added {
                        crate::msg::desktop_documents_toast_linked(&row.text)
                    } else {
                        crate::msg::desktop_documents_toast_unlinked(&row.text)
                    },
                );
            }
            Purpose::File(id) => {
                let Some(doc_type) = state.doc_type else {
                    self.chrome.status_message =
                        Some(crate::msg::desktop_documents_picker_need_type());
                    return;
                };
                self.close_documents_dialog();
                self.documents_file_by_hand(id, doc_type, row.link, cx);
            }
            Purpose::Follow(_) => {
                self.close_documents_dialog();
                if let Some(link) = row.link {
                    self.documents_goto_link(link, cx);
                }
            }
        }
    }

    /// `L`: follows a Link. In the Library one Link jumps straight there, several open a small list
    /// of them, and none is a quiet no-op. In the Inbox it follows the row's Suggested Link, so it
    /// can be checked before `y`.
    pub(super) fn documents_follow_link(&mut self, cx: &mut Context<'_, Self>) {
        let Some(document) = self.documents_selected_document(cx) else {
            return;
        };
        let id = document.id;
        match self.documents_state(cx).mode {
            DocumentsMode::Library => match document.links.as_slice() {
                [] => {}
                [only] => {
                    let only = *only;
                    self.documents_goto_link(only, cx);
                }
                _ => {
                    let state = PickerState::new(Purpose::Follow(id), document.date, None);
                    self.open_documents_picker(state, cx);
                }
            },
            DocumentsMode::Inbox => {
                let suggestion = documents::suggestion(
                    document,
                    self.documents(cx),
                    self.transactions(cx),
                    &self.payees,
                );
                if let Some(best) = suggestion.best() {
                    let transaction_id = best.transaction_id;
                    self.documents_goto_link(DocumentLink::Transaction(transaction_id), cx);
                }
            }
        }
    }

    /// Hands off to the record a Link points at, with it selected. There is no back-history: `g f`
    /// returns to Documents.
    pub(super) fn documents_goto_link(&mut self, link: DocumentLink, cx: &mut Context<'_, Self>) {
        match link {
            DocumentLink::Transaction(id) => self.open_transaction_row(id, cx),
            DocumentLink::Account(id) => {
                self.open_settings_page(SettingsSection::Accounts, cx);
                self.select_account(id, cx);
            }
            DocumentLink::Payee(id) => {
                self.open_settings_page(SettingsSection::Payees, cx);
                self.select_payee(id);
            }
            DocumentLink::BillPlan(id) => {
                let at = crate::bills::planner_order(self.bill_plans(cx))
                    .iter()
                    .position(|plan| plan.id == id);
                let Some(at) = at else {
                    return;
                };
                self.nav.set_noun(Noun::Bills);
                self.set_bills_tab(crate::bills::BillsTab::Planner, cx);
                self.edit_bills_state(cx, |state| state.selected = at);
            }
            DocumentLink::InventoryItem(_) => {
                self.nav.set_noun(Noun::Inventory);
                self.reset_view_scroll(cx);
            }
        }
    }

    /// A click on a LINKED TO name: follows that Link.
    fn handle_documents_link_click(&mut self, link: DocumentLink, cx: &mut Context<'_, Self>) {
        self.documents_goto_link(link, cx);
        cx.notify();
    }

    /// A click on a LINKED TO row's ×: removes the Link at once, with a Toast and no confirm.
    fn handle_documents_unlink_click(&mut self, link: DocumentLink, cx: &mut Context<'_, Self>) {
        if let Some(id) = self
            .documents_selected_document(cx)
            .map(|document| document.id)
        {
            let name = self.documents_link_name(link, cx);
            if self.mutate_documents(cx, |data| {
                documents::toggle_link(&mut data.documents, id, link)
            }) == Some(false)
            {
                self.raise_toast(
                    ToastKind::Info,
                    crate::msg::desktop_documents_toast_unlinked(&name),
                );
            }
        }
        cx.notify();
    }

    /// The text a Link reads as in the picker, for toasts.
    fn documents_link_name(&self, link: DocumentLink, cx: &gpui::App) -> String {
        let id = self
            .documents_selected_document(cx)
            .map(|document| document.id)
            .unwrap_or_default();
        let state = PickerState::new(Purpose::Follow(id), self.today, None);
        let lookups = self.documents_lookups(cx);
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
    pub(super) fn documents_accept_focused(&mut self, cx: &mut gpui::App) {
        let Some(document) = self.documents_selected_document(cx) else {
            return;
        };
        let id = document.id;
        let suggestion = documents::suggestion(
            document,
            self.documents(cx),
            self.transactions(cx),
            &self.payees,
        );
        match suggestion.best() {
            Some(best) => self.documents_accept(id, best.transaction_id, cx),
            None => self.documents_link_picker(cx),
        }
    }

    /// Files `id` under `transaction_id`, remembers it for `u` and moves focus to the next row.
    pub(super) fn documents_accept(&mut self, id: u32, transaction_id: u32, cx: &mut gpui::App) {
        let before = self.documents_before_filing(id, cx);
        // Copied out so the Documents can be written while the Transactions are read.
        let transactions = self.transactions(cx).to_vec();
        let Ok(entry) = self.mutate_documents(cx, |data| {
            documents::accept(
                &data.types,
                &mut data.documents,
                &transactions,
                id,
                transaction_id,
            )
        }) else {
            return;
        };
        self.documents_after_filing(id, before, entry, cx);
    }

    /// Files `id` by hand with a Document Type and at most one Link, as one filing action.
    fn documents_file_by_hand(
        &mut self,
        id: u32,
        doc_type: documents::DocumentType,
        link: Option<DocumentLink>,
        cx: &mut gpui::App,
    ) {
        let before = self.documents_before_filing(id, cx);
        let Ok(entry) = self.mutate_documents(cx, |data| {
            documents::file_by_hand(&mut data.documents, id, doc_type, link)
        }) else {
            return;
        };
        self.documents_after_filing(id, before, entry, cx);
    }

    /// Where `id` sits in the Inbox and what it is called, read before it leaves the Inbox.
    fn documents_before_filing(&self, id: u32, cx: &gpui::App) -> (Option<usize>, usize, String) {
        let rows = self.documents_inbox_rows(cx);
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
    fn documents_after_filing(
        &mut self,
        id: u32,
        (at, len, title): (Option<usize>, usize, String),
        entry: documents::FiledEntry,
        cx: &mut gpui::App,
    ) {
        self.edit_documents_state(cx, |state| {
            state.undo = Some(documents::FilingUndo {
                entries: vec![entry],
                focus: Some(id),
            });
        });
        if let Some(at) = at {
            let next = documents::focus_after_filing(at, len).unwrap_or(0);
            self.edit_documents_state(cx, |state| state.inbox_selected = next);
            self.documents_state(cx).scroll.scroll_to_item(next);
        }
        self.raise_toast(
            ToastKind::Success,
            crate::msg::desktop_documents_toast_filed(&title),
        );
    }

    /// `x` / **Skip**: sets the focused Document aside to the bottom of the Inbox for the session.
    /// Focus stays at the same position, which is now the next row.
    pub(super) fn documents_skip_focused(&mut self, cx: &mut gpui::App) {
        let Some(id) = self
            .documents_selected_document(cx)
            .map(|document| document.id)
        else {
            return;
        };
        if self
            .mutate_documents(cx, |data| documents::skip(&mut data.documents, id))
            .is_ok()
        {
            let len = self.documents_inbox_rows(cx).len();
            let at = self.documents_inbox_at(len, cx);
            self.edit_documents_state(cx, |state| state.inbox_selected = at);
        }
    }

    /// `u`: reverses the last filing action as a unit and puts focus back on its Document.
    pub(super) fn documents_undo_last(&mut self, cx: &mut gpui::App) {
        let Some(record) = self.edit_documents_state(cx, |state| state.undo.take()) else {
            self.chrome.status_message =
                Some(crate::msg::desktop_documents_status_nothing_to_undo());
            return;
        };
        let count = record.entries.len();
        let focus = self.mutate_documents(cx, |data| documents::undo(&mut data.documents, record));
        self.set_documents_mode(DocumentsMode::Inbox, cx);
        if let Some(id) = focus
            && let Some(at) = self
                .documents_inbox_rows(cx)
                .iter()
                .position(|document| document.id == id)
        {
            self.edit_documents_state(cx, |state| state.inbox_selected = at);
            self.documents_state(cx).scroll.scroll_to_item(at);
        }
        self.raise_toast(
            ToastKind::Info,
            crate::msg::desktop_documents_toast_undone(i64::try_from(count).unwrap_or(i64::MAX)),
        );
    }

    /// **Watched folder…**: out of scope for now, so it says so.
    pub(super) fn documents_watched_folder(&mut self) {
        self.raise_toast(
            ToastKind::Info,
            crate::msg::desktop_documents_watched_folder_toast(),
        );
    }

    /// `Y`, **Accept all strong matches** and `:documents accept-all`: asks first, with the count.
    pub(super) fn open_documents_accept_all(&mut self, cx: &gpui::App) {
        let count =
            documents::strong_count(self.documents(cx), self.transactions(cx), &self.payees);
        if count == 0 {
            self.chrome.status_message = Some(crate::msg::desktop_documents_status_no_strong());
            return;
        }
        self.open_documents_dialog(DocumentsDialog::AcceptAll(count));
    }

    pub(super) fn apply_documents_accept_all(&mut self, cx: &mut gpui::App) {
        let focus = self
            .documents_selected_document(cx)
            .map(|document| document.id);
        // Copied out so the Documents can be written while the Transactions are read.
        let transactions = self.transactions(cx).to_vec();
        let batch = self.mutate_documents(cx, |data| {
            documents::accept_all_strong(
                &data.types,
                &mut data.documents,
                &transactions,
                &self.payees,
                focus,
            )
        });
        let Some(batch) = batch else {
            return;
        };
        let count = batch.entries.len();
        self.edit_documents_state(cx, |state| state.undo = Some(batch));
        self.edit_documents_state(cx, |state| state.inbox_selected = 0);
        self.documents_state(cx)
            .scroll
            .set_offset(gpui::Point::default());
        self.raise_toast(
            ToastKind::Success,
            crate::msg::desktop_documents_toast_filed_many(&count.to_string()),
        );
    }

    /// `e` in the Inbox: the focused Document's Extracted Facts.
    fn open_documents_facts(&mut self, cx: &gpui::App) {
        let options = self.documents_options(cx);
        let Some(document) = self.documents_selected_document(cx) else {
            self.chrome.status_message = Some(crate::msg::desktop_documents_status_no_document());
            return;
        };
        let facts = document
            .intake
            .as_ref()
            .map(|intake| intake.facts.clone())
            .unwrap_or_default();
        let id = document.id;
        let form = FactsForm::new(&facts, &options, self.today, self.settings_date_style);
        self.open_documents_dialog(DocumentsDialog::Facts(id, Box::new(form)));
    }

    /// Saves the facts; suggestions recompute on the next render. A no-op while a field is wrong.
    pub(super) fn apply_documents_facts(&mut self, id: u32, form: &FactsForm, cx: &mut gpui::App) {
        let Some(facts) = form.build() else {
            return;
        };
        let title = documents::get(self.documents(cx), id)
            .map(|document| document.title.clone())
            .unwrap_or_default();
        if self
            .mutate_documents(cx, |data| {
                documents::edit_facts(&mut data.documents, id, facts)
            })
            .is_err()
        {
            return;
        }
        // Editing clears a skip, so the Document may have moved up the list: follow it.
        if let Some(at) = self
            .documents_inbox_rows(cx)
            .iter()
            .position(|document| document.id == id)
        {
            self.edit_documents_state(cx, |state| state.inbox_selected = at);
            self.documents_state(cx).scroll.scroll_to_item(at);
        }
        self.raise_toast(
            ToastKind::Success,
            crate::msg::desktop_documents_toast_facts_saved(&title),
        );
    }

    fn handle_facts_field_click(&mut self, field: FactsField, cx: &mut Context<'_, Self>) {
        if let Some(DocumentsDialog::Facts(_, form)) = self.documents_dialog_mut() {
            if FactsForm::is_select(field) {
                form.click_select();
            } else {
                form.focus(field);
            }
        }
        cx.notify();
    }

    fn handle_facts_option_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        if let Some(DocumentsDialog::Facts(_, form)) = self.documents_dialog_mut() {
            form.choose(index);
        }
        cx.notify();
    }

    // -----------------------------------------------------------------------------------------
    // Commands
    // -----------------------------------------------------------------------------------------

    /// A `documents <verb>` palette command.
    pub(super) fn run_documents_command(
        &mut self,
        verb: crate::navigation::command::DocumentsVerb,
        cx: &mut gpui::App,
    ) {
        use crate::navigation::command::DocumentsVerb;
        let noun_before = self.nav.noun();
        self.nav.set_noun(Noun::Documents);
        if noun_before != Noun::Documents {
            self.reset_view_scroll(cx);
        }
        match verb {
            DocumentsVerb::Inbox => self.set_documents_mode(DocumentsMode::Inbox, cx),
            DocumentsVerb::Library => self.set_documents_mode(DocumentsMode::Library, cx),
            DocumentsVerb::AcceptAll => {
                self.set_documents_mode(DocumentsMode::Inbox, cx);
                self.open_documents_accept_all(cx);
            }
            DocumentsVerb::Add => self.open_documents_add(cx),
            DocumentsVerb::Import => self.open_documents_import(),
        }
    }

    // -----------------------------------------------------------------------------------------
    // Dialogs
    // -----------------------------------------------------------------------------------------

    pub(super) fn open_documents_add(&mut self, cx: &gpui::App) {
        let options = self.documents_options(cx);
        self.open_documents_dialog(DocumentsDialog::Add(Box::new(DocumentForm::new(
            &options,
            self.documents_state(cx).scope,
            self.today,
            self.settings_date_style,
        ))));
    }

    pub(super) fn open_documents_import(&mut self) {
        self.open_documents_dialog(DocumentsDialog::Import(ImportForm::default(), Vec::new()));
    }

    fn open_documents_edit(&mut self, cx: &gpui::App) {
        let options = self.documents_options(cx);
        let Some(document) = self.documents_selected_document(cx) else {
            self.chrome.status_message = Some(crate::msg::desktop_documents_status_no_document());
            return;
        };
        let id = document.id;
        let form = DocumentForm::for_edit(document, &options, self.today, self.settings_date_style);
        self.open_documents_dialog(DocumentsDialog::Edit(id, Box::new(form)));
    }

    pub(super) fn close_documents_dialog(&mut self) {
        self.close_dialog();
    }

    pub(super) fn documents_dialog(&self) -> Option<&DocumentsDialog> {
        match self.chrome.dialog.as_ref()? {
            OpenDialog::Documents(dialog) => Some(dialog),
            _ => None,
        }
    }

    fn documents_dialog_mut(&mut self) -> Option<&mut DocumentsDialog> {
        match self.chrome.dialog.as_mut()? {
            OpenDialog::Documents(dialog) => Some(dialog),
            _ => None,
        }
    }

    fn open_documents_dialog(&mut self, dialog: DocumentsDialog) {
        self.open_dialog(OpenDialog::Documents(dialog));
    }

    /// Applies a confirmed Documents dialog (`Enter`, `Ctrl-Enter` in Import, and the confirm
    /// button). A form with a problem only `Shell` can see (a missing file, a duplicate path) and
    /// an import that took nothing reopen the Dialog, which shows why.
    pub(super) fn apply_documents_dialog(
        &mut self,
        dialog: DocumentsDialog,
        cx: &mut Context<'_, Self>,
    ) {
        match dialog {
            DocumentsDialog::Add(form) => self.apply_documents_form(None, form, cx),
            DocumentsDialog::Edit(id, form) => self.apply_documents_form(Some(id), form, cx),
            DocumentsDialog::Import(form, _) => self.apply_documents_import(form, cx),
            DocumentsDialog::Facts(id, form) => self.apply_documents_facts(id, &form, cx),
            DocumentsDialog::AcceptAll(_) => self.apply_documents_accept_all(cx),
            DocumentsDialog::Picker(state) => self.apply_documents_picker(*state, cx),
        }
    }

    /// A picker key that needed the live rows: moves the highlight over them, or acts on the
    /// highlighted one.
    fn apply_documents_picker(&mut self, mut state: PickerState, cx: &mut Context<'_, Self>) {
        let Some(request) = state.request.take() else {
            return;
        };
        let rows = self.documents_picker_rows(&state, cx).len();
        let at = state.selected.min(rows.saturating_sub(1));
        match request {
            PickerRequest::Pick => {
                self.open_documents_dialog(DocumentsDialog::Picker(Box::new(state)));
                self.documents_picker_pick(at, cx);
            }
            PickerRequest::Step { down } => {
                state.step(down, rows);
                self.open_documents_dialog(DocumentsDialog::Picker(Box::new(state)));
            }
        }
    }

    fn documents_form_mut(&mut self) -> Option<&mut DocumentForm> {
        match self.documents_dialog_mut() {
            Some(DocumentsDialog::Add(form) | DocumentsDialog::Edit(_, form)) => Some(form),
            _ => None,
        }
    }

    /// **Add document** / **Save** and `enter`: adds a Filed Document (selecting it) or saves the
    /// edit. A no-op while the form has a problem or is incomplete.
    pub(super) fn apply_documents_form(
        &mut self,
        editing: Option<u32>,
        form: Box<DocumentForm>,
        cx: &mut gpui::App,
    ) {
        let problems = form.problems(self.documents(cx), exists_on_disk);
        let Some(new) = form.can_save(&problems).then(|| form.build()).flatten() else {
            let dialog = match editing {
                None => DocumentsDialog::Add(form),
                Some(id) => DocumentsDialog::Edit(id, form),
            };
            self.open_documents_dialog(dialog);
            return;
        };
        let title = new.title.clone();
        match editing {
            None => {
                let Ok(id) = self
                    .mutate_documents(cx, |data| documents::add_filed(&mut data.documents, new))
                else {
                    self.open_documents_dialog(DocumentsDialog::Add(form));
                    return;
                };
                self.select_added_document(id, cx);
                self.raise_toast(
                    ToastKind::Success,
                    crate::msg::desktop_documents_toast_added(&title),
                );
            }
            Some(id) => {
                if self
                    .mutate_documents(cx, |data| {
                        documents::edit_metadata(
                            &mut data.documents,
                            id,
                            &new.title,
                            new.doc_type,
                            new.date,
                            new.key_date,
                        )
                    })
                    .is_err()
                {
                    self.open_documents_dialog(DocumentsDialog::Edit(id, form));
                    return;
                }
                self.select_added_document(id, cx);
                self.raise_toast(
                    ToastKind::Success,
                    crate::msg::desktop_documents_toast_saved(&title),
                );
            }
        }
    }

    /// Puts the Library on All documents, clears the search and selects `id`, so a Document just
    /// added or edited is on screen whatever scope was showing.
    fn select_added_document(&mut self, id: u32, cx: &mut gpui::App) {
        let in_scope = self
            .documents(cx)
            .iter()
            .find(|document| document.id == id)
            .is_some_and(|document| {
                self.documents_state(cx)
                    .scope
                    .contains(document, self.document_types(cx))
            });
        if !in_scope {
            self.edit_documents_state(cx, |state| state.scope = LibraryScope::All);
        }
        self.set_documents_mode(DocumentsMode::Library, cx);
        self.edit_documents_state(cx, |state| state.focus = DocumentsFocus::List);
        self.edit_documents_state(cx, |state| state.query.clear());
        let at = self
            .documents_library_rows(cx)
            .iter()
            .position(|document| document.id == id)
            .unwrap_or(0);
        self.edit_documents_state(cx, |state| state.selected = at);
        self.documents_state(cx).scroll.scroll_to_item(at);
    }

    /// **Import**: every typed path that exists lands in the Inbox. When none does the dialog stays
    /// open and says why each failed.
    pub(super) fn apply_documents_import(&mut self, form: ImportForm, cx: &mut gpui::App) {
        let outcomes = self.mutate_documents(cx, |data| {
            documents::form::import_all(
                &data.types,
                &form,
                &mut data.documents,
                self.today,
                exists_on_disk,
            )
        });
        let imported = outcomes
            .iter()
            .filter(|outcome| matches!(outcome, ImportOutcome::Imported(_)))
            .count();
        let notes: Vec<String> = outcomes
            .iter()
            .filter_map(|outcome| match outcome {
                ImportOutcome::Imported(_) => None,
                ImportOutcome::NotFound(path) => {
                    Some(crate::msg::desktop_documents_error_path_missing(path))
                }
                ImportOutcome::Unsupported(path) => Some(format!(
                    "{path}: {}",
                    crate::msg::desktop_documents_error_unsupported()
                )),
                ImportOutcome::Already(title) => {
                    Some(crate::msg::desktop_documents_error_already(title))
                }
            })
            .collect();
        if imported == 0 {
            self.open_documents_dialog(DocumentsDialog::Import(form, notes));
            return;
        }
        let skipped = notes.len();
        self.raise_toast(
            ToastKind::Success,
            if skipped == 0 {
                crate::msg::desktop_documents_toast_imported(
                    i64::try_from(imported).unwrap_or(i64::MAX),
                )
            } else {
                crate::msg::desktop_documents_toast_import_partial(
                    &imported.to_string(),
                    &skipped.to_string(),
                )
            },
        );
        self.set_documents_mode(DocumentsMode::Inbox, cx);
    }

    /// Files dropped anywhere on the window land in the Inbox like `Import…`, but the page stays
    /// where it is: the Toast and the rail badge say what arrived.
    pub fn drop_documents(&mut self, paths: &[std::path::PathBuf], cx: &mut gpui::App) {
        let outcomes = self.mutate_documents(cx, |data| {
            documents::form::import_dropped(
                &data.types,
                paths,
                &mut data.documents,
                self.today,
                exists_on_disk,
            )
        });
        let imported = outcomes
            .iter()
            .filter(|outcome| matches!(outcome, ImportOutcome::Imported(_)))
            .count();
        let skipped = outcomes.len() - imported;
        if imported == 0 {
            if skipped > 0 {
                self.raise_toast(
                    ToastKind::Error,
                    crate::msg::desktop_documents_toast_import_partial("0", &skipped.to_string()),
                );
            }
            return;
        }
        let toast = if skipped == 0 {
            crate::msg::desktop_documents_toast_imported(
                i64::try_from(imported).unwrap_or(i64::MAX),
            )
        } else {
            crate::msg::desktop_documents_toast_import_partial(
                &imported.to_string(),
                &skipped.to_string(),
            )
        };
        self.raise_toast(ToastKind::Success, toast);
    }

    fn handle_documents_field_click(&mut self, field: DocumentField, cx: &mut Context<'_, Self>) {
        if let Some(form) = self.documents_form_mut() {
            if DocumentForm::is_select(field) {
                form.click_select(field);
            } else if field == DocumentField::Reminder {
                form.focus(field);
                form.toggle_reminder();
            } else {
                form.focus(field);
            }
        }
        cx.notify();
    }

    fn handle_documents_option_click(
        &mut self,
        field: DocumentField,
        index: usize,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(form) = self.documents_form_mut() {
            form.choose(field, index);
        }
        cx.notify();
    }

    /// The open Documents dialog as an overlay element, or `None` when none is open.
    pub(super) fn render_documents_dialog(
        &self,
        entity: &gpui::Entity<Shell>,
        cx: &gpui::App,
    ) -> Option<gpui::AnyElement> {
        let dialog = self.documents_dialog()?;
        let on_cancel: crate::dialog::OnClick = {
            let entity = entity.clone();
            Rc::new(move |_window: &mut Window, cx: &mut gpui::App| {
                entity.update(cx, |shell, cx| {
                    shell.close_documents_dialog();
                    cx.notify();
                });
            })
        };
        let on_confirm: crate::dialog::OnClick = {
            let entity = entity.clone();
            Rc::new(move |_window: &mut Window, cx: &mut gpui::App| {
                entity.update(cx, |shell, cx| {
                    shell.confirm_open_dialog(cx);
                    cx.notify();
                });
            })
        };
        match dialog {
            DocumentsDialog::Picker(state) => {
                let rows = self.documents_picker_rows(state, cx);
                let on_pick: dialogs::OnPickerRow = {
                    let entity = entity.clone();
                    Rc::new(move |index, _window, cx| {
                        entity.update(cx, |shell, cx| {
                            shell.documents_picker_pick(index, cx);
                            cx.notify();
                        });
                    })
                };
                Some(dialogs::render_picker(
                    dialogs::PickerProps {
                        state,
                        rows: &rows,
                        type_label: state
                            .doc_type
                            .map(|kind| view_model::type_label(self.document_types(cx), kind)),
                        on_pick,
                    },
                    cx,
                ))
            }
            DocumentsDialog::AcceptAll(count) => Some(dialogs::render_accept_all(
                *count, on_cancel, on_confirm, cx,
            )),
            DocumentsDialog::Facts(_, form) => {
                let problems = form.problems();
                let on_field_click: dialogs::OnFactsFieldClick = {
                    let entity = entity.clone();
                    Rc::new(move |field, _window, cx| {
                        entity.update(cx, |shell, cx| shell.handle_facts_field_click(field, cx));
                    })
                };
                let on_option_click: dialogs::OnFactsOptionClick = {
                    let entity = entity.clone();
                    Rc::new(move |index, _window, cx| {
                        entity.update(cx, |shell, cx| shell.handle_facts_option_click(index, cx));
                    })
                };
                Some(dialogs::render_facts(
                    dialogs::FactsProps {
                        form,
                        options: &form.options,
                        valid: form.can_save(&problems),
                        problems: &problems,
                        on_field_click,
                        on_option_click,
                        on_cancel,
                        on_confirm,
                    },
                    cx,
                ))
            }
            DocumentsDialog::Import(form, notes) => Some(dialogs::render_import(
                dialogs::ImportProps {
                    form,
                    notes: notes.clone(),
                    valid: form.can_save(),
                    on_cancel,
                    on_confirm,
                },
                cx,
            )),
            DocumentsDialog::Add(form) | DocumentsDialog::Edit(_, form) => {
                let problems = form.problems(self.documents(cx), exists_on_disk);
                let on_field_click: dialogs::OnFieldClick = {
                    let entity = entity.clone();
                    Rc::new(move |field, _window, cx| {
                        entity.update(cx, |shell, cx| {
                            shell.handle_documents_field_click(field, cx)
                        });
                    })
                };
                let on_option_click: dialogs::OnOptionClick = {
                    let entity = entity.clone();
                    Rc::new(move |field, index, _window, cx| {
                        entity.update(cx, |shell, cx| {
                            shell.handle_documents_option_click(field, index, cx)
                        });
                    })
                };
                Some(dialogs::render_form(
                    dialogs::FormProps {
                        form,
                        options: &form.options,
                        valid: form.is_complete() && form.can_save(&problems),
                        problems: &problems,
                        handlers: dialogs::FormHandlers {
                            on_field_click,
                            on_option_click,
                            on_cancel,
                            on_confirm,
                        },
                    },
                    cx,
                ))
            }
        }
    }
}
