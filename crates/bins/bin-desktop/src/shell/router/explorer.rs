//! The file explorer's click handlers and the file it chooses, of `Root`. An `impl Root` block,
//! not the explorer: its Presenter is `lib-shell`'s, held by the Shell's (`lib_shell::explorer`),
//! and `navigation::explorer` draws it and reads the disk for it.

use std::path::{Path, PathBuf};

use gpui::Context;
use lib_database::Storage;
use lib_shell::{ExplorerChoice, ExplorerFilter, ExplorerMode, ExplorerOutcome};
use lib_toast::ToastKind;

use super::{LedgerFile, Root};
use crate::navigation::explorer::FsLedgerFolders;
use crate::navigation::nav::Noun;
use lib_shell::NavInput;

impl Root {
    /// `:open` or `:new`: opens the file explorer in `mode`, its row list at the top.
    pub(super) fn open_explorer(&mut self, mode: ExplorerMode) {
        self.presenter.open_explorer(mode, &FsLedgerFolders);
        self.reset_explorer_scroll();
    }

    /// A file explorer row click (`explorer::OnEntryClick`), `click_count` `2` for a real
    /// double-click: a double-click on the selected `.pldb` row opens it at once (README's
    /// "double-click a `.pldb` row opens immediately").
    pub(super) fn handle_explorer_entry_click(
        &mut self,
        path: PathBuf,
        click_count: usize,
        cx: &mut Context<'_, Self>,
    ) {
        let outcome = self
            .presenter
            .explorer_click(&path, click_count, &FsLedgerFolders);
        self.explorer_outcome(outcome, cx);
    }

    /// A breadcrumb segment click (`explorer::OnBreadcrumbClick`).
    pub(super) fn handle_explorer_breadcrumb_click(
        &mut self,
        path: PathBuf,
        cx: &mut Context<'_, Self>,
    ) {
        let outcome = self.presenter.explorer_browse(path, &FsLedgerFolders);
        self.explorer_outcome(outcome, cx);
    }

    /// A footer checkbox click (`explorer::OnFilterToggle`). The Presenter keeps the filters for
    /// the next opening and the quit-time save.
    pub(super) fn handle_explorer_filter_toggle(
        &mut self,
        filter: ExplorerFilter,
        cx: &mut Context<'_, Self>,
    ) {
        self.presenter.explorer_toggle_filter(filter);
        cx.notify();
    }

    /// The explorer dialog's own Cancel button: closes without opening anything, leaving
    /// Command mode the same way its `back` key does.
    pub(super) fn handle_explorer_cancel(&mut self, cx: &mut Context<'_, Self>) {
        self.presenter.close_explorer();
        self.navigate(NavInput::ExitMode);
        cx.notify();
    }

    /// The explorer dialog's own Open or Create button.
    pub(super) fn handle_explorer_open(&mut self, cx: &mut Context<'_, Self>) {
        let outcome = self.presenter.explorer_confirm();
        self.explorer_outcome(outcome, cx);
    }

    /// Acts on what an input did to the explorer: a file it chose is opened or created, and a new
    /// folder puts the row list back at the top. Returns whether the window needs redrawing.
    pub(super) fn explorer_outcome(
        &mut self,
        outcome: ExplorerOutcome,
        cx: &mut Context<'_, Self>,
    ) -> bool {
        match outcome {
            ExplorerOutcome::Ignored => return false,
            ExplorerOutcome::Changed | ExplorerOutcome::Closed => {}
            ExplorerOutcome::Chosen(choice) => self.open_chosen(choice, cx),
        }
        self.keep_explorer_scroll();
        cx.notify();
        true
    }

    /// Opens or creates the file the explorer chose, telling its Presenter how it went: one that
    /// opened closes the explorer, one that failed leaves it open beside the Error Toast.
    fn open_chosen(&mut self, choice: ExplorerChoice, cx: &mut Context<'_, Self>) {
        match choice.mode {
            ExplorerMode::Open => self.open_ledger_file(&choice.path, cx),
            ExplorerMode::New => self.create_ledger_file(&choice.path, cx),
        }
    }

    /// Puts the row list back at the top once the explorer browses another folder, remembering
    /// the folder it is on.
    fn keep_explorer_scroll(&mut self) {
        let folder = self.presenter.explorer().map(|explorer| explorer.folder);
        if folder != self.state.explorer_folder {
            self.reset_explorer_scroll();
        }
    }

    fn reset_explorer_scroll(&mut self) {
        self.state.explorer_folder = self.presenter.explorer().map(|explorer| explorer.folder);
        self.state
            .explorer_scroll
            .set_offset(gpui::Point::default());
    }

    /// The file explorer's rows by name, in order, while it is open.
    #[doc(hidden)]
    pub fn explorer_rows_snapshot(&self) -> Option<Vec<String>> {
        self.presenter.explorer().map(|explorer| {
            explorer
                .entries
                .into_iter()
                .map(|entry| entry.name)
                .collect()
        })
    }

    /// `:new`'s File name field while its explorer is open: the name as typed, and the file it
    /// clashes with, which the field refuses inline.
    #[doc(hidden)]
    pub fn explorer_file_name_snapshot(&self) -> Option<(String, Option<String>)> {
        self.presenter
            .explorer()
            .filter(|explorer| explorer.mode == ExplorerMode::New)
            .map(|explorer| (explorer.file_name, explorer.name_taken))
    }

    /// Opens the Ledger file at `path` in place of the Open Ledger, if any, closing the explorer,
    /// and lands on the Dashboard saying so. The Open Ledger's writes land first, so reopening its own file reads
    /// them back; it is swapped out only once the new file has opened, so a file that won't open
    /// leaves it as it was, or none open, and the explorer open, and an Error Toast says why.
    /// Never creates a file.
    fn open_ledger_file(&mut self, path: &Path, cx: &mut Context<'_, Self>) {
        self.flush_ledger();
        match crate::open_ledger_file(path, self.presenter.clock().today()) {
            Ok(storage) => {
                self.presenter.explorer_chosen(true);
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
    /// made keeps what was open, or none, and the explorer open, and an Error Toast says why.
    fn create_ledger_file(&mut self, path: &Path, cx: &mut Context<'_, Self>) {
        self.flush_ledger();
        match crate::create_ledger_file(path, self.presenter.clock().today()) {
            Ok(storage) => {
                self.presenter.explorer_chosen(true);
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
