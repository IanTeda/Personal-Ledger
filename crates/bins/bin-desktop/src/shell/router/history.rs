//! The top bar's history buttons on `Root`: Back and Forward through the nouns visited, and Undo
//! with its History list. An `impl Root` block for the concern, not a module of the name.

use gpui::Context;

use super::Root;
use crate::navigation::nav::Noun;
use crate::shell::view::topbar::{HistoryPick, TopBarAction};
use crate::shell::view::window_size::WindowSize;
use lib_shell::NavInput;

impl Root {
    /// Closes the running input's History entry from `labels`, its `Notice::Changed`s, the
    /// Configuration it changed and the Open Ledger's rows it wrote (`crate::reversal::close`).
    /// Only `send` calls it, right after the input's commit (#1277).
    pub(crate) fn close_history(&mut self, labels: Vec<String>) {
        let ledger = self
            .open
            .as_ref()
            .and_then(|open| open.ports.take_reversal());
        if let Some(entry) = crate::reversal::close(labels, self.configuration.take(), ledger) {
            self.presenter.record(entry);
        }
    }

    /// A click on Back, Forward, Undo or the History list (`shell::view::topbar::TopBar`'s
    /// `on_action`). Back and Forward land the way a rail click does, scroll reset included.
    /// Undo steps one entry, and a click on a History row undoes or redoes through it, each as one
    /// input that closes the list.
    pub(super) fn handle_topbar_action(
        &mut self,
        action: TopBarAction,
        cx: &mut Context<'_, Self>,
    ) {
        let noun_before = self.nav().noun();
        match action {
            TopBarAction::Back => {
                self.navigate(NavInput::Back);
            }
            TopBarAction::Forward => {
                self.navigate(NavInput::Forward);
            }
            TopBarAction::Undo => {
                self.close_undo_menu();
                self.send_with_router(WindowSize::Large, cx, |presenter, router| {
                    presenter.undo_through(1, router);
                });
            }
            TopBarAction::Pick(pick) => {
                self.close_undo_menu();
                self.send_with_router(WindowSize::Large, cx, |presenter, router| match pick {
                    HistoryPick::Undo(steps) => presenter.undo_through(steps, router),
                    HistoryPick::Redo(steps) => presenter.redo_through(steps, router),
                });
            }
            // Leaving a row clears only its own highlight, whichever order the pointer leaving
            // one row and entering the next arrive in.
            TopBarAction::Hover(pick, true) => self.state.undo_hover = Some(pick),
            TopBarAction::Hover(pick, false) => {
                if self.state.undo_hover == Some(pick) {
                    self.state.undo_hover = None;
                }
            }
            TopBarAction::ToggleUndoMenu => {
                if self.state.undo_menu_open {
                    self.close_undo_menu();
                } else {
                    self.state.undo_menu_open = true;
                }
            }
            TopBarAction::DismissUndoMenu => {
                self.close_undo_menu();
            }
        }
        if self.nav().noun() != noun_before {
            self.close_undo_menu();
            self.reset_view_scroll(cx);
        }
        // After the View is told it is entered, which lands Settings on its index: Back and Forward
        // show the page the history holds itself, since it records pages only. Only when Settings
        // is where they landed: restoring a hidden page would hand it the keyboard.
        if matches!(action, TopBarAction::Back | TopBarAction::Forward)
            && self.nav().noun() == Noun::Settings
        {
            let page = self.nav().settings_page();
            self.settings.update(cx, |view, cx| view.restore(page, cx));
        }
        cx.notify();
    }

    /// Closes the History list, forgetting the row the pointer was over.
    pub(super) fn close_undo_menu(&mut self) {
        self.state.undo_menu_open = false;
        self.state.undo_hover = None;
    }

    /// The History list below Undo is open.
    #[doc(hidden)]
    pub fn undo_menu_open(&self) -> bool {
        self.state.undo_menu_open
    }

    /// What the History list draws, from the Shell's Presenter.
    #[doc(hidden)]
    pub fn history(&self) -> lib_shell::HistoryViewModel {
        self.presenter.history()
    }

    /// The History row the pointer is over, whose click the list highlights the rows of.
    #[doc(hidden)]
    pub fn undo_hover(&self) -> Option<HistoryPick> {
        self.state.undo_hover
    }
}
