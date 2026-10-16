//! The status line's hint clicks and page status of `Shell`. An `impl Shell` block for the
//! concern, not the status line itself (`chrome::statusline`).

use crate::inventory::form::InventoryDialog;
use gpui::Context;

use super::{Shell, inventory_ui};
use crate::settings::hints::settings_inventory_hints;
use crate::view::active::ActiveViewEntity;
use crate::{
    chrome::dialog_host::Dialog,
    chrome::statusline::{HintAction, PageStatus},
    chrome::view_chrome::ViewChrome,
    navigation::nav::InputMode,
    settings::SettingsPage,
    settings::inventory as inventory_view,
};

impl Shell {
    /// The View on show as Chrome reads it: its title and its status-line content, built together
    /// so the top bar and the status line can't disagree about which View they describe.
    pub(super) fn view_chrome(&self, cx: &gpui::App) -> ViewChrome {
        ViewChrome {
            title: self.active_view(cx).title(),
            status: self.page_status(cx),
        }
    }

    /// A click on the status line's hint strip: the same action as the matching `Normal`-mode key,
    /// and ignored in any other mode (where those keys aren't live either).
    pub(super) fn handle_hint_click(&mut self, action: HintAction, cx: &mut Context<'_, Self>) {
        if self.nav.mode() != InputMode::Normal {
            return;
        }
        self.chrome.status_message = None;
        self.pending_g = None;
        match action {
            HintAction::Command => self.open_palette(),
            HintAction::Search => self.start_search(cx),
            HintAction::Help => self.nav.enter_mode(InputMode::Help),
            HintAction::ToggleRail => {
                self.nav.toggle_primary_rail();
                self.chrome.collapsed_rail_tooltip = None;
            }
        }
        cx.notify();
    }

    /// The status line's per-page hints and right-hand text for the View on show, or `None` when
    /// it has none. While a Dialog that has its own hints is open, they replace the View's
    /// (ADR-0037).
    pub(super) fn page_status(&self, cx: &gpui::App) -> Option<PageStatus> {
        let status = match self.active_view_entity(cx) {
            ActiveViewEntity::Shell(_) => None,
            view => self
                .settings_data_page_status(cx)
                .or_else(|| view.page_status(cx)),
        };
        match self.chrome.dialog.as_ref().and_then(Dialog::hints) {
            Some(hints) => Some(PageStatus {
                hints,
                right: status.map(|status| status.right).unwrap_or_default(),
            }),
            None => status,
        }
    }

    /// [`Self::page_status`] for a Settings page with Ledger data, which `Shell` still draws, or
    /// `None` when no such page has focus.
    fn settings_data_page_status(&self, cx: &gpui::App) -> Option<PageStatus> {
        let focused = |section| self.settings_data_page_has_focus(section, cx);
        if focused(SettingsPage::Inventory) {
            Some(PageStatus {
                hints: self.dialog::<InventoryDialog>().map_or_else(
                    || settings_inventory_hints(self.settings_inventory_selected_row(cx)),
                    inventory_ui::dialog_hints,
                ),
                right: inventory_view::scope_text(self.inventory(cx)),
            })
        } else {
            None
        }
    }
}
