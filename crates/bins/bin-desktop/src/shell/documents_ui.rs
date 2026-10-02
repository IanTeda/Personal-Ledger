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
        DocumentField, DocumentForm, DocumentOptions, DocumentsDialog, ImportForm, ImportOutcome,
    },
    key_router::Movement,
    nav::{FocusZone, InputMode, Noun},
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
    ]
}

fn index_hints() -> Vec<(&'static str, String)> {
    vec![
        ("j/k", crate::msg::desktop_hint_scope()),
        ("l/enter", crate::msg::desktop_hint_list()),
        ("i", crate::msg::desktop_hint_inbox()),
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

    /// The Document the Library's selection is on, if the Library is showing.
    fn documents_selected_document(&self) -> Option<&Document> {
        if self.documents_mode != DocumentsMode::Library {
            return None;
        }
        let rows = self.documents_library_rows();
        let at = self.documents_selected.min(rows.len().saturating_sub(1));
        rows.get(at).copied()
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
        let on_link_click: documents_view::OnLinkClick = {
            let entity = entity.clone();
            Rc::new(move |link, _window, cx| {
                entity.update(cx, |shell, cx| shell.handle_documents_link_click(link, cx));
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
            inbox_count: documents::inbox_count(&self.documents),
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
            on_add_link_click: plain(|shell, cx| {
                shell.documents_link_picker();
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
            DocumentsMode::Inbox => inbox_hints(),
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
            RailEntry::Inbox => self.documents_mode = DocumentsMode::Inbox,
            RailEntry::Scope(scope) => {
                self.documents_mode = DocumentsMode::Library;
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
        self.documents_selected = index;
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
        self.documents_mode = match self.documents_mode {
            DocumentsMode::Library => DocumentsMode::Inbox,
            DocumentsMode::Inbox => DocumentsMode::Library,
        };
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
        if self.documents_mode != DocumentsMode::Library {
            return;
        }
        let len = self.documents_library_rows().len();
        if len == 0 {
            return;
        }
        let current = self.documents_selected.min(len - 1);
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
        self.documents_selected = next;
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
                    self.status_message =
                        Some(crate::msg::desktop_documents_status_not_yet_built());
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
            "h" | "left" if in_list => {
                self.documents_focus = DocumentsFocus::Index;
                true
            }
            "o" if in_list && library => {
                self.documents_open_selected(true);
                true
            }
            "y" | "x" | "u" if !library => {
                self.status_message = Some(crate::msg::desktop_documents_status_not_yet_built());
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

    /// `l`, **+ Link to…** and a LINKED TO row: the picker and link navigation are the next
    /// ticket's work, so for now they say so.
    pub(super) fn documents_link_picker(&mut self) {
        self.status_message = Some(crate::msg::desktop_documents_status_not_yet_built());
    }

    fn handle_documents_link_click(&mut self, _link: DocumentLink, cx: &mut Context<'_, Self>) {
        self.documents_link_picker();
        cx.notify();
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
            DocumentsVerb::Inbox => self.documents_mode = DocumentsMode::Inbox,
            DocumentsVerb::Library => self.documents_mode = DocumentsMode::Library,
            DocumentsVerb::AcceptAll => {
                self.documents_mode = DocumentsMode::Inbox;
                self.status_message = Some(crate::msg::desktop_documents_status_not_yet_built());
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
            DocumentsDialog::Import(..) => return,
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
        self.documents_mode = DocumentsMode::Library;
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
        self.documents_mode = DocumentsMode::Inbox;
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
                    if matches!(shell.documents_dialog, Some(DocumentsDialog::Import(..))) {
                        shell.confirm_documents_import();
                    } else {
                        shell.confirm_documents_form();
                    }
                    cx.notify();
                });
            })
        };
        match dialog {
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
