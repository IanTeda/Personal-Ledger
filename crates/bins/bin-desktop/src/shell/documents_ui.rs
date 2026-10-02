//! `Shell`'s side of the Documents destination: the Library's scope, search, sort and selection,
//! the index-rail and list focus zones, the Add, Import and Edit dialogs, opening a file in the OS
//! and the page's status legend. A child of `shell` so it reads `Shell`'s private fields, without
//! growing that file further. The rules themselves are `documents` and `documents_form`; this only
//! wires them to keys and clicks.

use std::path::Path;
use std::rc::Rc;

use gpui::{Context, Keystroke, ScrollHandle, Window};
use lib_toast::ToastKind;

use super::{Shell, typed_char};
use crate::{
    accounts::SelectKey,
    documents::{
        self, Document, DocumentLink, DocumentsMode, KeyDateKind, LibraryScope, LibrarySort,
        RailEntry,
    },
    documents_form::{
        DocumentField, DocumentForm, DocumentOptions, DocumentsDialog, FactsField, FactsForm,
        ImportForm, ImportOutcome,
    },
    documents_picker::{self, PickerRow, PickerState, Purpose},
    key_router::Movement,
    nav::{FocusZone, InputMode, Noun},
    settings::SettingsSection,
    statusline::PageStatus,
    view::documents::{
        self as documents_view, DocumentsFocus, DocumentsPageProps, dialogs,
        model::{self as view_model, Lookups},
    },
};

/// `ctrl-d` / `ctrl-u` on the Library list: rows per half page.
const DOCUMENTS_HALF_PAGE: usize = 5;

/// What the key-down listener does with a file once it has an `App`: open it in the system viewer,
/// or reveal it in the file manager.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileAction {
    Open(std::path::PathBuf),
    Reveal(std::path::PathBuf),
}

fn type_label_for_options(kind: documents::DocumentType) -> String {
    view_model::type_label(kind)
}

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

/// What the integration tests read back of the Documents page: ids and plain values, so the
/// private `documents` types need not become public for them.
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
    /// Which dialog is open: `add`, `edit`, `import`, `facts`, `accept-all`, or
    /// `picker-link`, `picker-file`, `picker-follow`.
    pub dialog: Option<&'static str>,
    pub library_rows: usize,
    pub inbox_rows: usize,
    pub toasts: Vec<String>,
    pub status: Option<String>,
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
            dialog: self.documents_dialog.as_ref().map(|dialog| match dialog {
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
                .toasts
                .visible()
                .iter()
                .map(|toast| toast.text().to_string())
                .collect(),
            status: self.status_message.clone(),
        }
    }

    pub(super) fn documents_lookups(&self) -> Lookups<'_> {
        Lookups {
            accounts: &self.accounts,
            payees: &self.payees,
            plans: &self.bill_plans,
            inventory: &self.inventory,
            transactions: &self.transactions,
            today: self.today,
            date_style: self.settings_date_style,
        }
    }

    fn documents_options(&self) -> DocumentOptions {
        DocumentOptions::new(type_label_for_options, key_label_for_options)
    }

    /// The Library's rows under the current scope, search and sort.
    fn documents_library_rows(&self) -> Vec<&Document> {
        documents::library_rows(
            &self.documents,
            self.documents_scope,
            &self.documents_query,
            self.documents_sort,
            self.today,
        )
    }

    fn documents_rail_selected(&self) -> RailEntry {
        match self.documents_mode {
            DocumentsMode::Inbox => RailEntry::Inbox,
            DocumentsMode::Library => RailEntry::Scope(self.documents_scope),
        }
    }

    /// The Inbox's rows: newest received first, Skipped Documents last.
    fn documents_inbox_rows(&self) -> Vec<&Document> {
        documents::inbox_rows(&self.documents)
    }

    /// The Inbox row focus is on, a position clamped to the rows.
    fn documents_inbox_at(&self, len: usize) -> usize {
        self.documents_inbox_selected.min(len.saturating_sub(1))
    }

    /// The Document the selection is on: the Library's row or the Inbox's focused row.
    fn documents_selected_document(&self) -> Option<&Document> {
        match self.documents_mode {
            DocumentsMode::Library => {
                let rows = self.documents_library_rows();
                let at = self.documents_selected.min(rows.len().saturating_sub(1));
                rows.get(at).copied()
            }
            DocumentsMode::Inbox => {
                let rows = self.documents_inbox_rows();
                let at = self.documents_inbox_at(rows.len());
                rows.get(at).copied()
            }
        }
    }

    /// Switches Inbox / Library, putting the list back at its top when the mode changes.
    fn set_documents_mode(&mut self, mode: DocumentsMode) {
        if self.documents_mode != mode {
            self.documents_mode = mode;
            self.documents_scroll.set_offset(gpui::Point::default());
        }
    }

    fn documents_reset_selection(&mut self) {
        self.documents_selected = 0;
        self.documents_scroll.set_offset(gpui::Point::default());
    }

    pub(super) fn set_documents_persisted(
        &mut self,
        mode: DocumentsMode,
        scope: LibraryScope,
        sort: LibrarySort,
    ) {
        self.documents_mode = mode;
        self.documents_scope = scope;
        self.documents_sort = sort;
    }

    pub fn documents_persisted(&self) -> (DocumentsMode, LibraryScope, LibrarySort) {
        (
            self.documents_mode,
            self.documents_scope,
            self.documents_sort,
        )
    }

    /// Builds the page's props, only while Documents is the noun on show.
    pub(super) fn documents_page_props(&self, entity: &gpui::Entity<Shell>) -> DocumentsPageProps {
        let lookups = self.documents_lookups();
        let rows = self.documents_library_rows();
        let selected = self.documents_selected.min(rows.len().saturating_sub(1));
        let total = documents::scope_count(&self.documents, LibraryScope::All);
        let count_line = crate::msg::desktop_documents_count_line(
            &rows.len().to_string(),
            &total.to_string(),
            &documents::need_review_count(&rows, self.today).to_string(),
        );
        let detail = rows
            .get(selected)
            .filter(|_| self.documents_mode == DocumentsMode::Library)
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
        let inbox = self.documents_inbox_rows();
        let inbox_selected = self.documents_inbox_at(inbox.len());
        let inbox_views: Vec<_> = inbox
            .iter()
            .map(|document| view_model::inbox_row(document, &self.documents, &lookups))
            .collect();
        let inbox_detail = inbox
            .get(inbox_selected)
            .filter(|_| self.documents_mode == DocumentsMode::Inbox)
            .map(|document| view_model::inbox_detail(document, &self.documents, &lookups));
        let on_accept_row_click: documents_view::OnRowClick = {
            let entity = entity.clone();
            Rc::new(move |index, _window, cx| {
                entity.update(cx, |shell, cx| {
                    shell.documents_inbox_selected = index;
                    shell.documents_focus = DocumentsFocus::List;
                    shell.documents_accept_focused();
                    cx.notify();
                });
            })
        };
        let on_accept_candidate_click: documents_view::OnCandidateClick = {
            let entity = entity.clone();
            Rc::new(move |transaction_id, _window, cx| {
                entity.update(cx, |shell, cx| {
                    if let Some(id) = shell.documents_selected_document().map(|d| d.id) {
                        shell.documents_accept(id, transaction_id);
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
            mode: self.documents_mode,
            focus: self.documents_focus,
            rail: view_model::rail_rows(&self.documents, self.today),
            rail_selected: self.documents_rail_selected(),
            rail_footer: self.documents_stored_text(),
            subline: match self.documents_mode {
                DocumentsMode::Library => {
                    crate::msg::desktop_documents_subline(i64::try_from(total).unwrap_or(i64::MAX))
                }
                DocumentsMode::Inbox => crate::msg::desktop_documents_inbox_subline(
                    i64::try_from(documents::inbox_count(&self.documents)).unwrap_or(i64::MAX),
                ),
            },
            query: self.documents_query.clone(),
            searching: self.nav.mode() == InputMode::Search,
            sort: self.documents_sort,
            count_line,
            rows: row_views,
            selected,
            detail,
            inbox_rows: inbox_views,
            inbox_selected,
            inbox_detail,
            inbox_strong: documents::strong_count(
                &self.documents,
                &self.transactions,
                &self.payees,
            ),
            inbox_hints: inbox_hints(),
            list_focused: false,
            scroll: self.documents_scroll.clone(),
            on_rail_click,
            on_row_click,
            on_sort_click,
            on_search_click: plain(|shell, cx| {
                shell.documents_start_search();
                cx.notify();
            }),
            on_add_click: plain(|shell, cx| {
                shell.open_documents_add();
                cx.notify();
            }),
            on_import_click: plain(|shell, cx| {
                shell.open_documents_import();
                cx.notify();
            }),
            on_open_click: plain(|shell, cx| {
                shell.documents_open_selected(false);
                cx.notify();
            }),
            on_show_click: plain(|shell, cx| {
                shell.documents_open_selected(true);
                cx.notify();
            }),
            on_link_click,
            on_unlink_click,
            on_add_link_click: plain(|shell, cx| {
                shell.documents_link_picker();
                cx.notify();
            }),
            on_watched_click: plain(|shell, cx| {
                shell.documents_watched_folder();
                cx.notify();
            }),
            on_accept_all_click: plain(|shell, cx| {
                shell.open_documents_accept_all();
                cx.notify();
            }),
            on_accept_row_click,
            on_accept_next_click: plain(|shell, cx| {
                shell.documents_accept_focused();
                cx.notify();
            }),
            on_accept_candidate_click,
            on_link_elsewhere_click: plain(|shell, cx| {
                shell.documents_link_picker();
                cx.notify();
            }),
            on_skip_click: plain(|shell, cx| {
                shell.documents_skip_focused();
                cx.notify();
            }),
        }
    }

    /// "1.8 GB · stored beside household.pldb": the summed stub file sizes and the Ledger file.
    fn documents_stored_text(&self) -> String {
        crate::msg::desktop_documents_status_stored(
            &view_model::size_text(documents::total_bytes(&self.documents)),
            crate::statusline::STUB_LEDGER_FILE,
        )
    }

    /// The Documents page's status line for the focus zone and mode, or `None` off the page.
    pub(super) fn documents_page_status(&self) -> Option<PageStatus> {
        if self.nav.noun() != Noun::Documents {
            return None;
        }
        let hints = match &self.documents_dialog {
            Some(DocumentsDialog::Import(..)) => dialog_hints(true),
            Some(DocumentsDialog::Picker(_)) => picker_hints(),
            Some(_) => dialog_hints(false),
            None if self.documents_focus == DocumentsFocus::Index => index_hints(),
            None if self.documents_mode == DocumentsMode::Inbox => inbox_hints(),
            None => library_hints(),
        };
        Some(PageStatus {
            hints,
            right: self.documents_stored_text(),
        })
    }

    /// The `?` cheat-sheet's Documents group as `(action, keys)`: the index keys, then the current
    /// mode's. Empty off the page.
    pub(super) fn documents_cheat_sheet(&self) -> Vec<(String, &'static str)> {
        if self.nav.noun() != Noun::Documents {
            return Vec::new();
        }
        let mode = match self.documents_mode {
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

    fn select_documents_entry(&mut self, entry: RailEntry) {
        match entry {
            RailEntry::Inbox => self.set_documents_mode(DocumentsMode::Inbox),
            RailEntry::Scope(scope) => {
                self.set_documents_mode(DocumentsMode::Library);
                if self.documents_scope != scope {
                    self.documents_scope = scope;
                    self.documents_reset_selection();
                }
            }
        }
    }

    fn handle_documents_rail_click(&mut self, entry: RailEntry, cx: &mut Context<'_, Self>) {
        self.select_documents_entry(entry);
        self.documents_focus = DocumentsFocus::Index;
        cx.notify();
    }

    fn handle_documents_row_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        if self.documents_mode == DocumentsMode::Inbox {
            self.documents_inbox_selected = index;
        } else {
            self.documents_selected = index;
        }
        self.documents_focus = DocumentsFocus::List;
        cx.notify();
    }

    fn handle_documents_sort_click(&mut self, sort: LibrarySort, cx: &mut Context<'_, Self>) {
        if self.documents_sort != sort {
            self.documents_sort = sort;
            self.documents_reset_selection();
        }
        cx.notify();
    }

    /// The Library's `/`: focuses the toolbar search from either zone. Inert in the Inbox, which
    /// draws no search box.
    pub(super) fn documents_start_search(&mut self) {
        if self.documents_mode != DocumentsMode::Library {
            return;
        }
        self.documents_focus = DocumentsFocus::List;
        self.nav.enter_mode(InputMode::Search);
    }

    /// `i`: toggles Inbox and Library, keeping focus in its zone.
    fn toggle_documents_mode(&mut self) {
        self.set_documents_mode(match self.documents_mode {
            DocumentsMode::Library => DocumentsMode::Inbox,
            DocumentsMode::Inbox => DocumentsMode::Library,
        });
    }

    /// `j`/`k`/`gg`/`G`/`ctrl-d`/`ctrl-u`/`enter` on the Documents page, by focus zone.
    pub(super) fn apply_documents_movement(&mut self, movement: Movement) {
        match self.documents_focus {
            DocumentsFocus::Index => self.apply_documents_rail_movement(movement),
            DocumentsFocus::List => self.apply_documents_list_movement(movement),
        }
    }

    fn apply_documents_rail_movement(&mut self, movement: Movement) {
        let entries = documents::rail_entries(self.today);
        let current = entries
            .iter()
            .position(|entry| *entry == self.documents_rail_selected())
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
                self.documents_focus = DocumentsFocus::List;
                return;
            }
        };
        if let Some(entry) = entries.get(next) {
            self.select_documents_entry(*entry);
        }
    }

    fn apply_documents_list_movement(&mut self, movement: Movement) {
        if movement == Movement::Enter {
            self.documents_open_selected(false);
            return;
        }
        let inbox = self.documents_mode == DocumentsMode::Inbox;
        let len = if inbox {
            self.documents_inbox_rows().len()
        } else {
            self.documents_library_rows().len()
        };
        if len == 0 {
            return;
        }
        let current = if inbox {
            self.documents_inbox_at(len)
        } else {
            self.documents_selected.min(len - 1)
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
            self.documents_inbox_selected = next;
        } else {
            self.documents_selected = next;
        }
        self.documents_scroll.scroll_to_item(next);
    }

    // -----------------------------------------------------------------------------------------
    // Keys
    // -----------------------------------------------------------------------------------------

    /// The Documents keys that sit ahead of the global router (`a` would otherwise enter Insert
    /// mode), in `Normal` mode with the view focused. `false` for any key it does not own.
    pub(super) fn handle_documents_key(&mut self, keystroke: &Keystroke) -> bool {
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
        let library = self.documents_mode == DocumentsMode::Library;
        let in_list = self.documents_focus == DocumentsFocus::List;
        match keystroke.key.as_str() {
            "escape" => {
                // The innermost thing first: a committed query, then the list back to the index.
                if !self.documents_query.is_empty() {
                    self.documents_query.clear();
                    self.documents_reset_selection();
                    true
                } else if in_list {
                    self.documents_focus = DocumentsFocus::Index;
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
                self.toggle_documents_mode();
                true
            }
            // Library only; swallowed in the Inbox so `a` does not enter Insert mode there.
            "a" if !modifiers.shift => {
                if library {
                    self.open_documents_add();
                }
                true
            }
            "s" if library && !modifiers.shift => {
                self.documents_sort = self.documents_sort.toggle();
                self.documents_reset_selection();
                true
            }
            "e" if !modifiers.shift => {
                if library {
                    self.open_documents_edit();
                } else {
                    self.open_documents_facts();
                }
                true
            }
            "l" | "right" if !in_list => {
                self.documents_focus = DocumentsFocus::List;
                true
            }
            "l" if in_list && !modifiers.shift => {
                self.documents_link_picker();
                true
            }
            "l" if in_list && modifiers.shift => {
                self.documents_follow_link();
                true
            }
            "h" | "left" if in_list => {
                self.documents_focus = DocumentsFocus::Index;
                true
            }
            "o" if in_list => {
                self.documents_open_selected(true);
                true
            }
            // Inbox only; in the Library these fall through to the global router.
            "y" if !library && modifiers.shift => {
                self.open_documents_accept_all();
                true
            }
            "y" if !library && in_list => {
                self.documents_accept_focused();
                true
            }
            "x" if !library && in_list && !modifiers.shift => {
                self.documents_skip_focused();
                true
            }
            "u" if !library && !modifiers.shift => {
                self.documents_undo_last();
                true
            }
            _ => false,
        }
    }

    /// Keys while `InputMode::Search` is on the Documents page: typing narrows live, `Backspace`
    /// edits, `Enter` keeps the query and returns to the list. `Esc` is handled with the other
    /// modes' exit, which clears it.
    pub(super) fn handle_documents_search_key(&mut self, keystroke: &Keystroke) -> bool {
        match keystroke.key.as_str() {
            "enter" => {
                self.nav.exit_mode();
                true
            }
            "backspace" => {
                self.documents_query.pop();
                self.documents_reset_selection();
                true
            }
            _ => {
                let modifiers = &keystroke.modifiers;
                if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                    return false;
                }
                match typed_char(keystroke) {
                    Some(ch) if !ch.is_control() => {
                        self.documents_query.push(ch);
                        self.documents_reset_selection();
                        true
                    }
                    _ => false,
                }
            }
        }
    }

    /// `esc` out of Search: leaves the mode and clears the query.
    pub(super) fn documents_cancel_search(&mut self) {
        self.documents_query.clear();
        self.documents_reset_selection();
    }

    // -----------------------------------------------------------------------------------------
    // Files and links
    // -----------------------------------------------------------------------------------------

    /// `enter` / **Open** (`reveal` false) and `o` / **Show in folder** (`reveal` true): hands the
    /// file to the OS when it exists, and says so when it does not.
    pub(super) fn documents_open_selected(&mut self, reveal: bool) {
        let Some(document) = self.documents_selected_document() else {
            self.status_message = Some(crate::msg::desktop_documents_status_no_document());
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
        self.pending_file_action = Some(if reveal {
            FileAction::Reveal(path)
        } else {
            FileAction::Open(path)
        });
    }

    /// `l`, **+ Link to…**, **Link elsewhere…** and **File…**: opens the one link picker, to toggle
    /// a Link in the Library or to file the focused Unfiled Document from the Inbox.
    pub(super) fn documents_link_picker(&mut self) {
        let Some(document) = self.documents_selected_document() else {
            self.status_message = Some(crate::msg::desktop_documents_status_no_document());
            return;
        };
        let id = document.id;
        let state = match self.documents_mode {
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
        self.open_documents_picker(state);
    }

    fn open_documents_picker(&mut self, state: PickerState) {
        self.documents_dialog = Some(DocumentsDialog::Picker(state));
        self.nav.enter_mode(InputMode::Dialog);
    }

    /// The picker's rows for `state`: derived afresh from the stubs and the Document's Links.
    pub(super) fn documents_picker_rows(&self, state: &PickerState) -> Vec<PickerRow> {
        let lookups = self.documents_lookups();
        let current: Vec<DocumentLink> = documents::get(&self.documents, state.purpose.document())
            .map(|document| document.links.clone())
            .unwrap_or_default();
        documents_picker::rows(
            state,
            &documents_picker::Sources {
                lookups: &lookups,
                current: &current,
            },
        )
    }

    /// Keys while the picker is open, following the command palette's: typing goes to the query,
    /// `↑`/`↓` (and `ctrl+n`/`ctrl+p`) move, `tab` cycles the kind filter, `enter` picks. In the
    /// Inbox `←`/`→` step the Document Type select.
    fn handle_documents_picker_key(&mut self, keystroke: &Keystroke) -> bool {
        let Some(DocumentsDialog::Picker(state)) = self.documents_dialog.clone() else {
            return false;
        };
        let len = self.documents_picker_rows(&state).len();
        let modifiers = keystroke.modifiers;
        let Some(DocumentsDialog::Picker(live)) = self.documents_dialog.as_mut() else {
            return false;
        };
        match keystroke.key.as_str() {
            "enter" => {
                let at = state.selected.min(len.saturating_sub(1));
                self.documents_picker_pick(at);
            }
            "down" => live.step(true, len),
            "up" => live.step(false, len),
            "n" if modifiers.control => live.step(true, len),
            "p" if modifiers.control => live.step(false, len),
            "tab" => live.cycle_kind(),
            "left" | "right" if matches!(live.purpose, Purpose::File(_)) => {
                live.step_type(keystroke.key == "right");
            }
            "backspace" => live.backspace(),
            _ => {
                if let Some(ch) = typed_char(keystroke).filter(|ch| !ch.is_control()) {
                    live.push_char(ch);
                }
            }
        }
        true
    }

    /// What `enter` or a click on picker row `index` does: toggles a Link, files the Unfiled
    /// Document, or follows a Link, according to why the picker opened.
    pub(super) fn documents_picker_pick(&mut self, index: usize) {
        let Some(DocumentsDialog::Picker(state)) = self.documents_dialog.clone() else {
            return;
        };
        let Some(row) = self.documents_picker_rows(&state).get(index).cloned() else {
            return;
        };
        match state.purpose {
            Purpose::Link(id) => {
                let Some(link) = row.link else {
                    return;
                };
                let Some(added) = documents::toggle_link(&mut self.documents, id, link) else {
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
                    self.status_message = Some(crate::msg::desktop_documents_picker_need_type());
                    return;
                };
                self.close_documents_dialog();
                self.documents_file_by_hand(id, doc_type, row.link);
            }
            Purpose::Follow(_) => {
                self.close_documents_dialog();
                if let Some(link) = row.link {
                    self.documents_goto_link(link);
                }
            }
        }
    }

    /// `L`: follows a Link. In the Library one Link jumps straight there, several open a small list
    /// of them, and none is a quiet no-op. In the Inbox it follows the row's Suggested Link, so it
    /// can be checked before `y`.
    pub(super) fn documents_follow_link(&mut self) {
        let Some(document) = self.documents_selected_document() else {
            return;
        };
        let id = document.id;
        match self.documents_mode {
            DocumentsMode::Library => match document.links.as_slice() {
                [] => {}
                [only] => {
                    let only = *only;
                    self.documents_goto_link(only);
                }
                _ => {
                    let state = PickerState::new(Purpose::Follow(id), document.date, None);
                    self.open_documents_picker(state);
                }
            },
            DocumentsMode::Inbox => {
                let suggestion = documents::suggestion(
                    document,
                    &self.documents,
                    &self.transactions,
                    &self.payees,
                );
                if let Some(best) = suggestion.best() {
                    let transaction_id = best.transaction_id;
                    self.documents_goto_link(DocumentLink::Transaction(transaction_id));
                }
            }
        }
    }

    /// Hands off to the record a Link points at, with it selected. There is no back-history: `g f`
    /// returns to Documents.
    pub(super) fn documents_goto_link(&mut self, link: DocumentLink) {
        match link {
            DocumentLink::Transaction(id) => self.open_transaction_row(id),
            DocumentLink::Account(id) => {
                self.open_settings_page(SettingsSection::Accounts);
                self.select_account(id);
            }
            DocumentLink::Payee(id) => {
                self.open_settings_page(SettingsSection::Payees);
                self.select_payee(id);
            }
            DocumentLink::BillPlan(id) => {
                let at = crate::bills::planner_order(&self.bill_plans)
                    .iter()
                    .position(|plan| plan.id == id);
                let Some(at) = at else {
                    return;
                };
                self.nav.set_noun(Noun::Bills);
                self.set_bills_tab(crate::bills::BillsTab::Planner);
                self.bills_selected = at;
            }
            DocumentLink::InventoryItem(_) => {
                self.nav.set_noun(Noun::Inventory);
                self.reset_view_scroll();
            }
        }
    }

    /// A click on a LINKED TO name: follows that Link.
    fn handle_documents_link_click(&mut self, link: DocumentLink, cx: &mut Context<'_, Self>) {
        self.documents_goto_link(link);
        cx.notify();
    }

    /// A click on a LINKED TO row's ×: removes the Link at once, with a Toast and no confirm.
    fn handle_documents_unlink_click(&mut self, link: DocumentLink, cx: &mut Context<'_, Self>) {
        if let Some(id) = self
            .documents_selected_document()
            .map(|document| document.id)
        {
            let name = self.documents_link_name(link);
            if documents::toggle_link(&mut self.documents, id, link) == Some(false) {
                self.raise_toast(
                    ToastKind::Info,
                    crate::msg::desktop_documents_toast_unlinked(&name),
                );
            }
        }
        cx.notify();
    }

    /// The text a Link reads as in the picker, for toasts.
    fn documents_link_name(&self, link: DocumentLink) -> String {
        let id = self
            .documents_selected_document()
            .map(|document| document.id)
            .unwrap_or_default();
        let state = PickerState::new(Purpose::Follow(id), self.today, None);
        let lookups = self.documents_lookups();
        documents_picker::rows(
            &state,
            &documents_picker::Sources {
                lookups: &lookups,
                current: &[link],
            },
        )
        .into_iter()
        .next()
        .map(|row| row.text)
        .unwrap_or_default()
    }

    /// Runs a queued file action with an `App`, after the key handler has recorded it.
    pub(super) fn run_pending_file_action(&mut self, cx: &mut Context<'_, Self>) {
        let Some(action) = self.pending_file_action.take() else {
            return;
        };
        let (path, reveal) = match &action {
            FileAction::Open(path) => (path, false),
            FileAction::Reveal(path) => (path, true),
        };
        if reveal {
            cx.reveal_path(path);
        } else {
            cx.open_with_system(path);
        }
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.status_message = Some(if reveal {
            crate::msg::desktop_documents_status_shown(&name)
        } else {
            crate::msg::desktop_documents_status_opened(&name)
        });
    }

    // -----------------------------------------------------------------------------------------
    // Inbox: accept, skip, undo, accept all
    // -----------------------------------------------------------------------------------------

    /// `y` and **Accept & next**: files the focused Document under its Suggested Link. A Document
    /// with no suggestion is filed by hand instead, through the picker.
    pub(super) fn documents_accept_focused(&mut self) {
        let Some(document) = self.documents_selected_document() else {
            return;
        };
        let id = document.id;
        let suggestion =
            documents::suggestion(document, &self.documents, &self.transactions, &self.payees);
        match suggestion.best() {
            Some(best) => self.documents_accept(id, best.transaction_id),
            None => self.documents_link_picker(),
        }
    }

    /// Files `id` under `transaction_id`, remembers it for `u` and moves focus to the next row.
    pub(super) fn documents_accept(&mut self, id: u32, transaction_id: u32) {
        let before = self.documents_before_filing(id);
        let Ok(entry) =
            documents::accept(&mut self.documents, &self.transactions, id, transaction_id)
        else {
            return;
        };
        self.documents_after_filing(id, before, entry);
    }

    /// Files `id` by hand with a Document Type and at most one Link, as one filing action.
    fn documents_file_by_hand(
        &mut self,
        id: u32,
        doc_type: documents::DocumentType,
        link: Option<DocumentLink>,
    ) {
        let before = self.documents_before_filing(id);
        let Ok(entry) = documents::file_by_hand(&mut self.documents, id, doc_type, link) else {
            return;
        };
        self.documents_after_filing(id, before, entry);
    }

    /// Where `id` sits in the Inbox and what it is called, read before it leaves the Inbox.
    fn documents_before_filing(&self, id: u32) -> (Option<usize>, usize, String) {
        let rows = self.documents_inbox_rows();
        (
            rows.iter().position(|document| document.id == id),
            rows.len(),
            documents::get(&self.documents, id)
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
    ) {
        self.documents_undo = Some(documents::FilingUndo {
            entries: vec![entry],
            focus: Some(id),
        });
        if let Some(at) = at {
            self.documents_inbox_selected = documents::focus_after_filing(at, len).unwrap_or(0);
            self.documents_scroll
                .scroll_to_item(self.documents_inbox_selected);
        }
        self.raise_toast(
            ToastKind::Success,
            crate::msg::desktop_documents_toast_filed(&title),
        );
    }

    /// `x` / **Skip**: sets the focused Document aside to the bottom of the Inbox for the session.
    /// Focus stays at the same position, which is now the next row.
    pub(super) fn documents_skip_focused(&mut self) {
        let Some(id) = self
            .documents_selected_document()
            .map(|document| document.id)
        else {
            return;
        };
        if documents::skip(&mut self.documents, id).is_ok() {
            let len = self.documents_inbox_rows().len();
            self.documents_inbox_selected = self.documents_inbox_at(len);
        }
    }

    /// `u`: reverses the last filing action as a unit and puts focus back on its Document.
    pub(super) fn documents_undo_last(&mut self) {
        let Some(record) = self.documents_undo.take() else {
            self.status_message = Some(crate::msg::desktop_documents_status_nothing_to_undo());
            return;
        };
        let count = record.entries.len();
        let focus = documents::undo(&mut self.documents, record);
        self.set_documents_mode(DocumentsMode::Inbox);
        if let Some(id) = focus
            && let Some(at) = self
                .documents_inbox_rows()
                .iter()
                .position(|document| document.id == id)
        {
            self.documents_inbox_selected = at;
            self.documents_scroll.scroll_to_item(at);
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
    pub(super) fn open_documents_accept_all(&mut self) {
        let count = documents::strong_count(&self.documents, &self.transactions, &self.payees);
        if count == 0 {
            self.status_message = Some(crate::msg::desktop_documents_status_no_strong());
            return;
        }
        self.documents_dialog = Some(DocumentsDialog::AcceptAll(count));
        self.nav.enter_mode(InputMode::Dialog);
    }

    pub(super) fn confirm_documents_accept_all(&mut self) {
        let focus = self
            .documents_selected_document()
            .map(|document| document.id);
        let batch = documents::accept_all_strong(
            &mut self.documents,
            &self.transactions,
            &self.payees,
            focus,
        );
        self.close_documents_dialog();
        let Some(batch) = batch else {
            return;
        };
        let count = batch.entries.len();
        self.documents_undo = Some(batch);
        self.documents_inbox_selected = 0;
        self.documents_scroll.set_offset(gpui::Point::default());
        self.raise_toast(
            ToastKind::Success,
            crate::msg::desktop_documents_toast_filed_many(&count.to_string()),
        );
    }

    /// `e` in the Inbox: the focused Document's Extracted Facts.
    fn open_documents_facts(&mut self) {
        let options = self.documents_options();
        let Some(document) = self.documents_selected_document() else {
            self.status_message = Some(crate::msg::desktop_documents_status_no_document());
            return;
        };
        let facts = document
            .intake
            .as_ref()
            .map(|intake| intake.facts.clone())
            .unwrap_or_default();
        let id = document.id;
        let form = FactsForm::new(&facts, &options, self.settings_date_style);
        self.documents_dialog = Some(DocumentsDialog::Facts(id, form));
        self.nav.enter_mode(InputMode::Dialog);
    }

    /// Saves the facts; suggestions recompute on the next render. A no-op while a field is wrong.
    pub(super) fn confirm_documents_facts(&mut self) {
        let options = self.documents_options();
        let (today, style) = (self.today, self.settings_date_style);
        let Some(DocumentsDialog::Facts(id, form)) = self.documents_dialog.as_ref() else {
            return;
        };
        if !form.can_save(&form.problems(today, style)) {
            return;
        }
        let Some(facts) = form.build(&options, today, style) else {
            return;
        };
        let id = *id;
        let title = documents::get(&self.documents, id)
            .map(|document| document.title.clone())
            .unwrap_or_default();
        if documents::edit_facts(&mut self.documents, id, facts).is_err() {
            return;
        }
        // Editing clears a skip, so the Document may have moved up the list: follow it.
        if let Some(at) = self
            .documents_inbox_rows()
            .iter()
            .position(|document| document.id == id)
        {
            self.documents_inbox_selected = at;
            self.documents_scroll.scroll_to_item(at);
        }
        self.close_documents_dialog();
        self.raise_toast(
            ToastKind::Success,
            crate::msg::desktop_documents_toast_facts_saved(&title),
        );
    }

    fn handle_facts_field_click(&mut self, field: FactsField, cx: &mut Context<'_, Self>) {
        let options = self.documents_options();
        if let Some(DocumentsDialog::Facts(_, form)) = self.documents_dialog.as_mut() {
            if FactsForm::is_select(field) {
                form.click_select(&options);
            } else {
                form.focus(field);
            }
        }
        cx.notify();
    }

    fn handle_facts_option_click(&mut self, index: usize, cx: &mut Context<'_, Self>) {
        let options = self.documents_options();
        if let Some(DocumentsDialog::Facts(_, form)) = self.documents_dialog.as_mut() {
            form.choose(index, &options);
        }
        cx.notify();
    }

    // -----------------------------------------------------------------------------------------
    // Commands
    // -----------------------------------------------------------------------------------------

    /// A `documents <verb>` palette command.
    pub(super) fn run_documents_command(&mut self, verb: crate::command::DocumentsVerb) {
        use crate::command::DocumentsVerb;
        let noun_before = self.nav.noun();
        self.nav.set_noun(Noun::Documents);
        if noun_before != Noun::Documents {
            self.reset_view_scroll();
        }
        match verb {
            DocumentsVerb::Inbox => self.set_documents_mode(DocumentsMode::Inbox),
            DocumentsVerb::Library => self.set_documents_mode(DocumentsMode::Library),
            DocumentsVerb::AcceptAll => {
                self.set_documents_mode(DocumentsMode::Inbox);
                self.open_documents_accept_all();
            }
            DocumentsVerb::Add => self.open_documents_add(),
            DocumentsVerb::Import => self.open_documents_import(),
        }
    }

    // -----------------------------------------------------------------------------------------
    // Dialogs
    // -----------------------------------------------------------------------------------------

    pub(super) fn open_documents_add(&mut self) {
        let options = self.documents_options();
        self.documents_dialog = Some(DocumentsDialog::Add(DocumentForm::new(
            &options,
            self.documents_scope,
            self.today,
            self.settings_date_style,
        )));
        self.nav.enter_mode(InputMode::Dialog);
    }

    pub(super) fn open_documents_import(&mut self) {
        self.documents_dialog = Some(DocumentsDialog::Import(ImportForm::default(), Vec::new()));
        self.nav.enter_mode(InputMode::Dialog);
    }

    fn open_documents_edit(&mut self) {
        let options = self.documents_options();
        let Some(document) = self.documents_selected_document() else {
            self.status_message = Some(crate::msg::desktop_documents_status_no_document());
            return;
        };
        let id = document.id;
        let form = DocumentForm::for_edit(document, &options, self.settings_date_style);
        self.documents_dialog = Some(DocumentsDialog::Edit(id, form));
        self.nav.enter_mode(InputMode::Dialog);
    }

    pub(super) fn close_documents_dialog(&mut self) {
        self.documents_dialog = None;
        self.nav.exit_mode();
    }

    /// The first `Esc` closes an open select list only; the next cancels the dialog.
    pub(super) fn documents_dialog_close_select(&mut self) -> bool {
        match self.documents_dialog.as_mut() {
            Some(DocumentsDialog::Add(form) | DocumentsDialog::Edit(_, form)) => {
                form.close_open_select()
            }
            Some(DocumentsDialog::Facts(_, form)) => form.close_open_select(),
            _ => false,
        }
    }

    fn documents_form_mut(&mut self) -> Option<&mut DocumentForm> {
        match self.documents_dialog.as_mut() {
            Some(DocumentsDialog::Add(form) | DocumentsDialog::Edit(_, form)) => Some(form),
            _ => None,
        }
    }

    /// Keys while a Documents dialog is open. `enter` on a select opens or commits its list and
    /// anywhere else saves (in Import it types a newline, and `ctrl+enter` saves); `space` flips the
    /// reminder checkbox. `Esc` never reaches here.
    pub(super) fn handle_documents_dialog_key(&mut self, keystroke: &Keystroke) -> bool {
        if matches!(self.documents_dialog, Some(DocumentsDialog::Picker(_))) {
            return self.handle_documents_picker_key(keystroke);
        }
        let options = self.documents_options();
        if let Some(DocumentsDialog::Import(form, notes)) = self.documents_dialog.as_mut() {
            match keystroke.key.as_str() {
                "enter" if keystroke.modifiers.control => self.confirm_documents_import(),
                "enter" => form.newline(),
                "backspace" => {
                    form.backspace();
                    notes.clear();
                }
                "tab" => {}
                _ => {
                    let modifiers = &keystroke.modifiers;
                    if modifiers.control || modifiers.alt || modifiers.platform {
                        return false;
                    }
                    if let Some(ch) = typed_char(keystroke) {
                        form.push_char(ch);
                        notes.clear();
                    }
                }
            }
            return true;
        }
        if let Some(DocumentsDialog::AcceptAll(_)) = self.documents_dialog.as_ref() {
            if keystroke.key == "enter" {
                self.confirm_documents_accept_all();
            }
            return true;
        }
        if let Some(DocumentsDialog::Facts(_, form)) = self.documents_dialog.as_mut() {
            let modifiers = keystroke.modifiers;
            match keystroke.key.as_str() {
                "tab" => form.cycle_focus(modifiers.shift, &options),
                "up" | "down" if form.focused == FactsField::Type => {
                    let key = if keystroke.key == "up" {
                        SelectKey::Up
                    } else {
                        SelectKey::Down
                    };
                    form.handle_select_key(key, &options);
                }
                "space" | "enter" if form.focused == FactsField::Type => {
                    form.handle_select_key(SelectKey::Activate, &options);
                }
                "enter" => self.confirm_documents_facts(),
                "backspace" => form.backspace(),
                _ => {
                    if modifiers.control
                        || modifiers.alt
                        || modifiers.platform
                        || modifiers.function
                    {
                        return false;
                    }
                    if let Some(ch) = typed_char(keystroke) {
                        form.push_char(ch);
                    }
                }
            }
            return true;
        }
        let Some(form) = self.documents_form_mut() else {
            return false;
        };
        let focused = form.focused;
        let on_select = DocumentForm::is_select(focused);
        let modifiers = keystroke.modifiers;
        match keystroke.key.as_str() {
            "tab" => form.cycle_focus(modifiers.shift, &options),
            "up" | "down" if on_select => {
                let key = if keystroke.key == "up" {
                    SelectKey::Up
                } else {
                    SelectKey::Down
                };
                form.handle_select_key(key, &options);
            }
            "space" | "enter" if on_select => {
                form.handle_select_key(SelectKey::Activate, &options);
            }
            "space" if focused == DocumentField::Reminder => form.toggle_reminder(),
            "enter" => self.confirm_documents_form(),
            "backspace" => form.backspace(),
            _ => {
                if modifiers.control || modifiers.alt || modifiers.platform || modifiers.function {
                    return false;
                }
                if let Some(ch) = typed_char(keystroke) {
                    form.push_char(ch);
                }
            }
        }
        true
    }

    /// **Add document** / **Save** and `enter`: adds a Filed Document (selecting it) or saves the
    /// edit. A no-op while the form has a problem or is incomplete.
    pub(super) fn confirm_documents_form(&mut self) {
        let options = self.documents_options();
        let (today, style) = (self.today, self.settings_date_style);
        let Some(dialog) = self.documents_dialog.as_ref() else {
            return;
        };
        let (editing, form) = match dialog {
            DocumentsDialog::Add(form) => (None, form),
            DocumentsDialog::Edit(id, form) => (Some(*id), form),
            DocumentsDialog::Import(..)
            | DocumentsDialog::Facts(..)
            | DocumentsDialog::Picker(_)
            | DocumentsDialog::AcceptAll(_) => return,
        };
        let problems = form.problems(&options, &self.documents, today, style, exists_on_disk);
        if !form.can_save(&problems) {
            return;
        }
        let Some(new) = form.build(&options, today, style) else {
            return;
        };
        let title = new.title.clone();
        match editing {
            None => {
                let Ok(id) = documents::add_filed(&mut self.documents, new) else {
                    return;
                };
                self.select_added_document(id);
                self.raise_toast(
                    ToastKind::Success,
                    crate::msg::desktop_documents_toast_added(&title),
                );
            }
            Some(id) => {
                if documents::edit_metadata(
                    &mut self.documents,
                    id,
                    &new.title,
                    new.doc_type,
                    new.date,
                    new.key_date,
                )
                .is_err()
                {
                    return;
                }
                self.select_added_document(id);
                self.raise_toast(
                    ToastKind::Success,
                    crate::msg::desktop_documents_toast_saved(&title),
                );
            }
        }
        self.close_documents_dialog();
    }

    /// Puts the Library on All documents, clears the search and selects `id`, so a Document just
    /// added or edited is on screen whatever scope was showing.
    fn select_added_document(&mut self, id: u32) {
        let in_scope = self
            .documents
            .iter()
            .find(|document| document.id == id)
            .is_some_and(|document| self.documents_scope.contains(document));
        if !in_scope {
            self.documents_scope = LibraryScope::All;
        }
        self.set_documents_mode(DocumentsMode::Library);
        self.documents_focus = DocumentsFocus::List;
        self.documents_query.clear();
        let at = self
            .documents_library_rows()
            .iter()
            .position(|document| document.id == id)
            .unwrap_or(0);
        self.documents_selected = at;
        self.documents_scroll.scroll_to_item(at);
    }

    /// **Import**: every typed path that exists lands in the Inbox. When none does the dialog stays
    /// open and says why each failed.
    pub(super) fn confirm_documents_import(&mut self) {
        let Some(DocumentsDialog::Import(form, _)) = self.documents_dialog.as_ref() else {
            return;
        };
        if !form.can_save() {
            return;
        }
        let form = form.clone();
        let outcomes = crate::documents_form::import_all(
            &form,
            &mut self.documents,
            self.today,
            exists_on_disk,
        );
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
            if let Some(DocumentsDialog::Import(_, shown)) = self.documents_dialog.as_mut() {
                *shown = notes;
            }
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
        self.set_documents_mode(DocumentsMode::Inbox);
        self.close_documents_dialog();
    }

    fn handle_documents_field_click(&mut self, field: DocumentField, cx: &mut Context<'_, Self>) {
        let options = self.documents_options();
        if let Some(form) = self.documents_form_mut() {
            if DocumentForm::is_select(field) {
                form.click_select(field, &options);
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
        let options = self.documents_options();
        if let Some(form) = self.documents_form_mut() {
            form.choose(field, index, &options);
        }
        cx.notify();
    }

    /// The open Documents dialog as an overlay element, or `None` when none is open.
    pub(super) fn render_documents_dialog(
        &self,
        entity: &gpui::Entity<Shell>,
        cx: &gpui::App,
    ) -> Option<gpui::AnyElement> {
        let dialog = self.documents_dialog.as_ref()?;
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
                    match shell.documents_dialog {
                        Some(DocumentsDialog::Import(..)) => shell.confirm_documents_import(),
                        Some(DocumentsDialog::AcceptAll(_)) => shell.confirm_documents_accept_all(),
                        Some(DocumentsDialog::Facts(..)) => shell.confirm_documents_facts(),
                        _ => shell.confirm_documents_form(),
                    }
                    cx.notify();
                });
            })
        };
        match dialog {
            DocumentsDialog::Picker(state) => {
                let rows = self.documents_picker_rows(state);
                let on_pick: dialogs::OnPickerRow = {
                    let entity = entity.clone();
                    Rc::new(move |index, _window, cx| {
                        entity.update(cx, |shell, cx| {
                            shell.documents_picker_pick(index);
                            cx.notify();
                        });
                    })
                };
                Some(dialogs::render_picker(
                    dialogs::PickerProps {
                        state,
                        rows: &rows,
                        type_label: state.doc_type.map(view_model::type_label),
                        on_pick,
                    },
                    cx,
                ))
            }
            DocumentsDialog::AcceptAll(count) => Some(dialogs::render_accept_all(
                *count, on_cancel, on_confirm, cx,
            )),
            DocumentsDialog::Facts(_, form) => {
                let options = self.documents_options();
                let problems = form.problems(self.today, self.settings_date_style);
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
                        options: &options,
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
                let options = self.documents_options();
                let problems = form.problems(
                    &options,
                    &self.documents,
                    self.today,
                    self.settings_date_style,
                    exists_on_disk,
                );
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
                        options: &options,
                        valid: form.can_save(&problems),
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

/// A fresh scroll handle for the Library list.
pub(super) fn new_scroll() -> ScrollHandle {
    ScrollHandle::new()
}
