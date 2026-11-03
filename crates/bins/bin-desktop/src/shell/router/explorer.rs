//! The file explorer's click handlers and open confirmation of `Root`. An `impl Root` block, not
//! the explorer model (`navigation::explorer`).

use std::path::{Path, PathBuf};

use gpui::Context;
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

    /// The explorer dialog's own Open button.
    pub(super) fn handle_explorer_open(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_explorer_open(cx);
    }

    /// Confirms the explorer's current selection and closes the dialog. `:open` opens the selected
    /// Ledger file ([`Self::open_ledger_file`]); `:new` still only raises a stand-in Toast and
    /// creates nothing, until it creates the file for real (#980). A no-op if nothing is selected
    /// (Open/New is only clickable once `FileExplorer::can_open` is true, but a double-click can
    /// also reach here -- see [`Self::handle_explorer_entry_click`] -- so this re-checks rather
    /// than trusting the caller).
    pub(super) fn confirm_explorer_open(&mut self, cx: &mut Context<'_, Self>) {
        let Some(explorer) = self.file_explorer.as_ref() else {
            return;
        };
        if !explorer.can_open() {
            return;
        }
        let Some(path) = explorer.selected().map(Path::to_path_buf) else {
            return;
        };
        let mode = explorer.mode();
        self.file_explorer = None;
        self.navigate(NavInput::ExitMode);
        if mode == ExplorerMode::Open {
            self.open_ledger_file(&path, cx);
        } else {
            self.raise_toast(
                ToastKind::Success,
                lib_locale::msg::toast_ledger_created(&file_name(&path)),
            );
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
                self.open_ledger(LedgerFile::opened(path.to_path_buf(), storage), cx);
                self.show_noun(Noun::Dashboard, cx);
                self.set_last_ledger(Some(path.to_path_buf()), cx);
                self.raise_toast(
                    ToastKind::Success,
                    lib_locale::msg::toast_ledger_opened(&file_name(path)),
                );
            }
            Err(failure) => self.ledger_open_failed(&failure),
        }
    }
}

/// `path`'s file name as a Toast names it, else the whole path.
pub(super) fn file_name(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}
