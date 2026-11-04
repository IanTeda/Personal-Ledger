//! The file explorer's click handlers and open confirmation of `Root`. An `impl Root` block, not
//! the explorer model (`navigation::explorer`).

use std::path::{Path, PathBuf};

use gpui::Context;
use lib_database::Storage;
use lib_toast::ToastKind;

use super::{LedgerFile, Root};
use crate::navigation::explorer::{ExplorerFilter, ExplorerMode};
use crate::navigation::nav::Noun;
use lib_shell::NavInput;

/// Where `:open`'s and `:new`'s file explorer starts browsing: the Ledger folder (`data`), as
/// Settings › General says, else -- before start-up has set it, or with no such folder on disk --
/// the platform home directory, falling back to the current directory on a platform/sandbox with
/// no resolvable home.
pub(super) fn explorer_start_dir(ledger_folder: Option<&Path>) -> PathBuf {
    ledger_folder
        .filter(|folder| folder.is_dir())
        .map(Path::to_path_buf)
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
}

impl Root {
    /// A file explorer row click (`explorer::OnEntryClick`): applies it to `FileExplorer`'s own
    /// state, then -- README's "double-click a `.pldb` row opens immediately" -- confirms the
    /// open immediately when `click_count` reports a real double-click landing on a row that
    /// (as of the resulting state) is the current selection. A single click on a not-yet-open
    /// `.pldb` row only selects it; a second, separate click completing the double-click is
    /// what actually opens it.
    pub(super) fn handle_explorer_entry_click(
        &mut self,
        path: PathBuf,
        click_count: usize,
        cx: &mut Context<'_, Self>,
    ) {
        let Some(explorer) = self.file_explorer.as_mut() else {
            return;
        };
        explorer.click_entry(&path);
        if click_count >= 2 && explorer.selected() == Some(path.as_path()) {
            self.confirm_explorer_open(cx);
            return;
        }
        cx.notify();
    }

    /// A breadcrumb segment click (`explorer::OnBreadcrumbClick`).
    pub(super) fn handle_explorer_breadcrumb_click(
        &mut self,
        path: PathBuf,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(explorer) = self.file_explorer.as_mut() {
            explorer.navigate_to(path);
            cx.notify();
        }
    }

    /// A footer checkbox click (`explorer::OnFilterToggle`): re-filters the open dialog and keeps
    /// the new state for the next one and for the quit-time save.
    pub(super) fn handle_explorer_filter_toggle(
        &mut self,
        filter: ExplorerFilter,
        cx: &mut Context<'_, Self>,
    ) {
        if let Some(explorer) = self.file_explorer.as_mut() {
            explorer.toggle_filter(filter);
            self.explorer_filters = explorer.filters();
            cx.notify();
        }
    }

    /// The explorer dialog's own Cancel button: closes without opening anything, leaving
    /// Command mode the same way the palette's own `esc` does.
    pub(super) fn handle_explorer_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.file_explorer = None;
        self.navigate(NavInput::ExitMode);
        cx.notify();
    }

    /// The file explorer's rows by name, in order, while it is open.
    #[doc(hidden)]
    pub fn explorer_rows_snapshot(&self) -> Option<Vec<String>> {
        self.file_explorer.as_ref().map(|explorer| {
            explorer
                .entries()
                .iter()
                .map(|entry| entry.name.clone())
                .collect()
        })
    }

    /// `:new`'s File name field while its explorer is open: the name as typed, and the file it
    /// clashes with, which the field refuses inline.
    #[doc(hidden)]
    pub fn explorer_file_name_snapshot(&self) -> Option<(String, Option<String>)> {
        self.file_explorer
            .as_ref()
            .filter(|explorer| explorer.mode() == ExplorerMode::New)
            .map(|explorer| (explorer.file_name().to_string(), explorer.name_taken()))
    }

    /// The explorer dialog's own Open or Create button.
    pub(super) fn handle_explorer_open(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_explorer_open(cx);
    }

    /// Confirms the explorer and closes it: `:open` opens the selected Ledger file
    /// ([`Self::open_ledger_file`]), `:new` creates the file its name makes in the folder browsed
    /// to and opens that ([`Self::create_ledger_file`]). A no-op while there is nothing to act on
    /// (Open/Create is only clickable once `FileExplorer::can_open` is true, but a double-click or
    /// `enter` can also reach here, so this re-checks rather than trusting the caller).
    pub(super) fn confirm_explorer_open(&mut self, cx: &mut Context<'_, Self>) {
        let Some(explorer) = self.file_explorer.as_ref() else {
            return;
        };
        let Some(path) = explorer.target() else {
            return;
        };
        let mode = explorer.mode();
        self.file_explorer = None;
        self.navigate(NavInput::ExitMode);
        match mode {
            ExplorerMode::Open => self.open_ledger_file(&path, cx),
            ExplorerMode::New => self.create_ledger_file(&path, cx),
        }
        cx.notify();
    }

    /// Opens the Ledger file at `path` in place of the Open Ledger, if any, and lands on the
    /// Dashboard saying so. The Open Ledger's writes land first, so reopening its own file reads
    /// them back; it is swapped out only once the new file has opened, so a file that won't open
    /// leaves it as it was, or none open, and an Error Toast says why. Never creates a file.
    fn open_ledger_file(&mut self, path: &Path, cx: &mut Context<'_, Self>) {
        self.flush_ledger();
        match crate::open_ledger_file(path, self.presenter.clock().today()) {
            Ok(storage) => {
                self.swap_in_ledger(path, storage, cx);
                self.raise_toast(
                    ToastKind::Success,
                    lib_locale::msg::toast_ledger_opened(&file_name(path)),
                );
            }
            Err(failure) => self.ledger_open_failed(&failure),
        }
    }

    /// Creates a Ledger file at `path` and opens it in place of the Open Ledger, if any, as
    /// [`Self::open_ledger_file`] does: swapped in only once it has opened, so a file that can't be
    /// made keeps what was open, or none, and an Error Toast says why.
    fn create_ledger_file(&mut self, path: &Path, cx: &mut Context<'_, Self>) {
        self.flush_ledger();
        match crate::create_ledger_file(path, self.presenter.clock().today()) {
            Ok(storage) => {
                self.swap_in_ledger(path, storage, cx);
                self.raise_toast(
                    ToastKind::Success,
                    lib_locale::msg::toast_ledger_created(&file_name(path)),
                );
            }
            Err(failure) => self.raise_toast(
                ToastKind::Error,
                lib_locale::msg::toast_ledger_create_failed(
                    &file_name(&failure.path),
                    &failure.reason,
                ),
            ),
        }
    }

    /// Makes the Ledger file at `path`, open on `storage`, the Open Ledger, landing on its
    /// Dashboard and remembering it for the next start.
    fn swap_in_ledger(&mut self, path: &Path, storage: Storage, cx: &mut Context<'_, Self>) {
        self.open_ledger(LedgerFile::opened(path.to_path_buf(), storage), cx);
        self.show_noun(Noun::Dashboard, cx);
        self.set_last_ledger(Some(path.to_path_buf()), cx);
    }
}

pub(super) use crate::navigation::explorer::display_name as file_name;
