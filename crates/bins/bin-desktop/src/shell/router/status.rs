//! The status line's hint clicks and page status of `Root`. An `impl Root` block for the
//! concern, not the status line itself (`shell::view::statusline`).

use gpui::Context;
use lib_shell::ShellCommand;

use super::Root;
use crate::{
    navigation::nav::InputMode,
    shell::view::statusline::{HintAction, PageStatus},
    shell::view::view_status::ViewStatus,
    shell::view::window_size::WindowSize,
};

impl Root {
    /// The View on show as the Shell reads it: its status-line content.
    pub(super) fn view_status(&self, cx: &gpui::App) -> ViewStatus {
        ViewStatus {
            status: self.page_status(cx),
        }
    }

    /// A click on the status line's hint strip: the same command as the matching `Normal`-mode
    /// key, run by the Shell's Presenter as one input, and ignored in any other mode (where those
    /// keys aren't live either).
    pub(super) fn handle_hint_click(
        &mut self,
        action: HintAction,
        window_size: WindowSize,
        cx: &mut Context<'_, Self>,
    ) {
        if self.nav().mode() != InputMode::Normal {
            return;
        }
        // A click ends a waiting `g` as its timer would.
        if self.prefix_timer.take().is_some() {
            self.send_shell(lib_shell::ShellInput::PrefixExpired, cx);
        }
        let command = match action {
            HintAction::Command => ShellCommand::Command,
            HintAction::Search => ShellCommand::Search,
            HintAction::Help => ShellCommand::Help,
            HintAction::ToggleRail => ShellCommand::ToggleRail,
        };
        self.send_with_router(window_size, cx, |presenter, router| {
            presenter.command(command, router)
        });
        cx.notify();
    }

    /// The status line's per-page hints and right-hand text for the View on show, or `None` when
    /// it has none. While a Dialog that has its own hints is open, they replace the View's
    /// (ADR-0037).
    pub(super) fn page_status(&self, cx: &gpui::App) -> Option<PageStatus> {
        let status = self.active_page(cx).and_then(|page| page.page_status(cx));
        let hints = self.hosted_dialog().and_then(|dialog| dialog.hints(cx));
        match hints {
            Some(hints) => Some(PageStatus {
                hints,
                right: status.map(|status| status.right).unwrap_or_default(),
            }),
            None => status,
        }
    }
}
