//! The status line's hint clicks and page status of `Root`. An `impl Root` block for the
//! concern, not the status line itself (`shell::view::statusline`).

use gpui::Context;

use super::Root;
use crate::view::active::ActiveViewEntity;
use crate::{
    navigation::nav::InputMode,
    shell::view::dialog_host::Dialog,
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

    /// A click on the status line's hint strip: the same action as the matching `Normal`-mode key,
    /// and ignored in any other mode (where those keys aren't live either).
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
        let hints = match self.hosted_dialog() {
            Some(dialog) => dialog.hints(&self.state.dialog_keymap, cx),
            None => self.state.dialog.as_ref().and_then(Dialog::hints),
        };
        match hints {
            Some(hints) => Some(PageStatus {
                hints,
                right: status.map(|status| status.right).unwrap_or_default(),
            }),
            None => status,
        }
    }
}
