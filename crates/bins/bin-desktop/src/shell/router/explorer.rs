//! The file explorer's click handlers of `Root`. An `impl Root` block, not the explorer: its
//! Presenter is `lib-shell`'s, held by the Shell's (`lib_shell::explorer`), the root Router opens
//! the file it chooses, and `navigation::explorer` draws it and reads the disk for it.

use std::path::{Path, PathBuf};

use gpui::Context;
use lib_shell::{ExplorerFilter, ExplorerMode, NavInput};

use super::Root;
use crate::shell::view::window_size::WindowSize;

impl Root {
    /// A file explorer row click (`explorer::OnEntryClick`), `click_count` `2` for a real
    /// double-click, as one input: a double-click on the selected `.pldb` row opens it at once
    /// (README's "double-click a `.pldb` row opens immediately"), through the root Router.
    pub(super) fn handle_explorer_entry_click(
        &mut self,
        path: PathBuf,
        click_count: usize,
        cx: &mut Context<'_, Self>,
    ) {
        self.send_with_router(WindowSize::Large, cx, |presenter, router| {
            presenter.explorer_click(&path, click_count, router)
        });
        cx.notify();
    }

    /// The Create ledger Dialog has written the Ledger file at `path`: opens it in place of the
    /// Open Ledger and, once it opened, closes the explorer and lands on the Dashboard, as the
    /// explorer's own pick of a file does, as one input. One that didn't open leaves the explorer
    /// up with the Error Toast.
    pub(super) fn ledger_created(&mut self, path: &Path, cx: &mut Context<'_, Self>) {
        self.send_with_router(WindowSize::Large, cx, |presenter, router| {
            let opened = router.open_created(path);
            presenter.explorer_chosen(opened, router);
        });
        cx.notify();
    }

    /// A breadcrumb segment click (`explorer::OnBreadcrumbClick`), as one input.
    pub(super) fn handle_explorer_breadcrumb_click(
        &mut self,
        path: PathBuf,
        cx: &mut Context<'_, Self>,
    ) {
        self.send_with_router(WindowSize::Large, cx, |presenter, router| {
            presenter.explorer_browse(path, router)
        });
        cx.notify();
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
        self.keep_explorer_scroll();
        cx.notify();
    }

    /// The explorer dialog's own Open or Create button, as one input: the file it chose opens
    /// through the root Router.
    pub(super) fn handle_explorer_open(&mut self, cx: &mut Context<'_, Self>) {
        self.send_with_router(WindowSize::Large, cx, |presenter, router| {
            presenter.explorer_confirm(router)
        });
        cx.notify();
    }

    /// Puts the row list back at the top once the explorer opens or browses another folder,
    /// remembering the folder it is on.
    pub(super) fn keep_explorer_scroll(&mut self) {
        let folder = self.presenter.explorer().map(|explorer| explorer.folder);
        if folder != self.state.explorer_folder {
            self.state.explorer_folder = folder;
            self.state
                .explorer_scroll
                .set_offset(gpui::Point::default());
        }
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
}

pub(super) use crate::navigation::explorer::display_name as file_name;
