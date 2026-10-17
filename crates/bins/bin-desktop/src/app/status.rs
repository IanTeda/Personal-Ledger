//! The status line's hint clicks and page status of `Shell`. An `impl Shell` block for the
//! concern, not the status line itself (`chrome::statusline`).

use gpui::Context;

use super::Shell;
use crate::view::active::ActiveViewEntity;
use crate::{
    chrome::dialog_host::Dialog,
    chrome::statusline::{HintAction, PageStatus},
    chrome::view_chrome::ViewChrome,
    chrome::window_size::WindowSize,
    navigation::nav::InputMode,
};

impl Shell {
    /// The View on show as Chrome reads it: its status-line content.
    pub(super) fn view_chrome(&self, cx: &gpui::App) -> ViewChrome {
        ViewChrome {
            status: self.page_status(cx),
        }
    }

    /// A click on the status line's hint strip: the same action as the matching `Normal`-mode key,
    /// and ignored in any other mode (where those keys aren't live either).
    pub(super) fn handle_hint_click(
        &mut self,
        action: HintAction,
        window_size: WindowSize,
        cx: &mut Context<'_, Self>,
    ) {
        if self.nav.mode() != InputMode::Normal {
            return;
        }
        self.chrome.status_message = None;
        self.pending_g = None;
        match action {
            HintAction::Command => self.open_palette(),
            HintAction::Search => self.start_search(cx),
            HintAction::Help => self.open_help(cx),
            HintAction::ToggleRail => self.toggle_rail(window_size, cx),
        }
        cx.notify();
    }

    /// The status line's per-page hints and right-hand text for the View on show, or `None` when
    /// it has none. While a Dialog that has its own hints is open, they replace the View's
    /// (ADR-0037).
    pub(super) fn page_status(&self, cx: &gpui::App) -> Option<PageStatus> {
        let status = match self.active_view_entity(cx) {
            ActiveViewEntity::Shell(_) => None,
            view => view.page_status(cx),
        };
        match self.chrome.dialog.as_ref().and_then(Dialog::hints) {
            Some(hints) => Some(PageStatus {
                hints,
                right: status.map(|status| status.right).unwrap_or_default(),
            }),
            None => status,
        }
    }
}
