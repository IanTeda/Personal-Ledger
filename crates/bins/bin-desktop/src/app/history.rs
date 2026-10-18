//! The top bar's history buttons on `Shell`: Back and Forward through the nouns visited, and Undo
//! with its recent-actions list. An `impl Shell` block for the concern, not a module of the name.

use gpui::Context;

use super::Shell;
use crate::chrome::topbar::TopBarAction;

impl Shell {
    /// A click on Back, Forward, Undo or the recent-actions list (`chrome::topbar::TopBar`'s
    /// `on_action`). Back and Forward land the way a rail click does, scroll reset included.
    /// Undo has no history to act on yet, so it says so on the status line.
    pub(super) fn handle_topbar_action(
        &mut self,
        action: TopBarAction,
        cx: &mut Context<'_, Self>,
    ) {
        let noun_before = self.nav.noun();
        match action {
            TopBarAction::Back => {
                self.nav.go_back();
            }
            TopBarAction::Forward => {
                self.nav.go_forward();
            }
            TopBarAction::Undo | TopBarAction::UndoTo(_) => {
                self.chrome.undo_menu_open = false;
                self.chrome.status_message = Some(crate::msg::desktop_status_undo_not_yet_built());
            }
            TopBarAction::ToggleUndoMenu => {
                self.chrome.undo_menu_open = !self.chrome.undo_menu_open;
            }
            TopBarAction::DismissUndoMenu => {
                self.chrome.undo_menu_open = false;
            }
        }
        if matches!(action, TopBarAction::Back | TopBarAction::Forward) {
            let page = self.nav.settings_page();
            self.views
                .settings
                .update(cx, |view, cx| view.set_selected_section(cx, page));
        }
        if self.nav.noun() != noun_before {
            self.chrome.undo_menu_open = false;
            self.reset_view_scroll(cx);
        }
        cx.notify();
    }

    /// The recent-actions list below Undo is open.
    #[doc(hidden)]
    pub fn undo_menu_open(&self) -> bool {
        self.chrome.undo_menu_open
    }
}
