//! The file explorer's click handlers and open confirmation of `Shell`. An `impl Shell` block, not
//! the explorer model (`navigation::explorer`).

use std::path::PathBuf;

use gpui::Context;
use lib_toast::ToastKind;

use super::Shell;
use crate::navigation::explorer::{ExplorerFilter, ExplorerMode};

/// Where `:open`'s file explorer starts browsing -- the handoff names no default starting
/// directory of its own, so the platform home directory is the reasonable stand-in, falling
/// back to the current directory on a platform/sandbox with no resolvable home.
pub(super) fn explorer_start_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

impl Shell {
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
        self.nav.exit_mode();
        cx.notify();
    }

    /// The explorer dialog's own Open button.
    pub(super) fn handle_explorer_open(&mut self, cx: &mut Context<'_, Self>) {
        self.confirm_explorer_open(cx);
    }

    /// Confirms the explorer's current selection and closes the dialog. In `ExplorerMode::Open`
    /// this is the stand-in "opening" effect (`NavState::open_ledger`, from issue #164) -- real
    /// `.pldb` parsing stays out of scope for this map. In `ExplorerMode::New` (issue #167) it
    /// closes without touching `NavState::ledger_open` at all: the dialog is a literal copy of
    /// Open's for now, but confirming an *existing* file was never what "new" means, even as a
    /// stand-in -- the real "create a fresh `.pldb`" workflow is still fog. A no-op if nothing
    /// is selected (Open/New is only clickable once `FileExplorer::can_open` is true, but a
    /// double-click can also reach here -- see [`Self::handle_explorer_entry_click`] -- so this
    /// re-checks rather than trusting the caller).
    pub(super) fn confirm_explorer_open(&mut self, cx: &mut Context<'_, Self>) {
        let Some(explorer) = self.file_explorer.as_ref() else {
            return;
        };
        if !explorer.can_open() {
            return;
        }
        let name = explorer
            .selected()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        // Neither mode can fail yet (no real `.pldb` I/O), so `toast-ledger-open-failed` waits
        // for the ticket that parses and creates ledger files.
        if explorer.mode() == ExplorerMode::Open {
            self.nav.open_ledger();
            self.raise_toast(
                ToastKind::Success,
                lib_locale::msg::toast_ledger_opened(&name),
            );
        } else {
            self.raise_toast(
                ToastKind::Success,
                lib_locale::msg::toast_ledger_created(&name),
            );
        }
        self.file_explorer = None;
        self.nav.exit_mode();
        cx.notify();
    }
}
