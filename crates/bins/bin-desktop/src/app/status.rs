//! The status line's hint clicks and page status of `Shell`. An `impl Shell` block for the
//! concern, not the status line itself (`chrome::statusline`).

use crate::documents::types::DocumentTypesDialog;
use crate::inventory::form::InventoryDialog;
use crate::payees::form::PayeesDialog;
use crate::tags::form::TagsDialog;
use gpui::Context;

use super::{Shell, document_types_ui, inventory_ui};
use crate::view::{
    accounts::hints::accounts_hints,
    active::{ActiveViewEntity, ShellView},
    settings::hints::{
        confirm_dialog_hints, merge_tags_dialog_hints, payee_dialog_hints,
        settings_categories_hints, settings_documents_hints, settings_inventory_hints,
        settings_payees_hints, settings_tags_hints, tag_dialog_hints,
    },
};
use crate::{
    chrome::dialog_host::Dialog,
    chrome::statusline::{HintAction, PageStatus},
    chrome::view_chrome::ViewChrome,
    navigation::nav::{FocusZone, InputMode},
    tags,
    view::settings::{self as settings_view, inventory as inventory_view},
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
            ActiveViewEntity::Shell(view) => self.shell_page_status(view, cx),
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

    /// [`Self::page_status`] for a View `Shell` still draws.
    fn shell_page_status(&self, view: ShellView, cx: &gpui::App) -> Option<PageStatus> {
        match view {
            ShellView::Settings(_) if self.accounts_page_has_focus(cx) => Some(PageStatus {
                hints: accounts_hints(),
                right: crate::msg::desktop_accounts_count(
                    i64::try_from(self.accounts.read(cx).accounts(cx).len()).unwrap_or(i64::MAX),
                ),
            }),
            ShellView::Settings(_) if self.settings_tags_page_has_focus(cx) => Some(PageStatus {
                hints: match self.dialog::<TagsDialog>() {
                    Some(TagsDialog::Add(_)) => tag_dialog_hints(false),
                    Some(TagsDialog::Edit(..)) => tag_dialog_hints(true),
                    Some(TagsDialog::Remove(..)) => confirm_dialog_hints(),
                    Some(TagsDialog::Merge(_)) => merge_tags_dialog_hints(),
                    None => settings_tags_hints(),
                },
                right: settings_view::tags::scope_text(
                    self.tags_list(cx),
                    &tags::duplicate_groups(self.tags_list(cx), self.transactions(cx)),
                ),
            }),
            ShellView::Settings(_) if self.settings_payees_page_has_focus(cx) => Some(PageStatus {
                hints: match self.dialog::<PayeesDialog>() {
                    Some(PayeesDialog::Delete(..)) => confirm_dialog_hints(),
                    Some(_) => payee_dialog_hints(),
                    None => settings_payees_hints(),
                },
                right: settings_view::payees::scope_text(self.payees_list(cx)),
            }),
            ShellView::Settings(_) if self.settings_documents_page_has_focus(cx) => {
                Some(PageStatus {
                    hints: self
                        .dialog::<DocumentTypesDialog>()
                        .map_or_else(settings_documents_hints, document_types_ui::dialog_hints),
                    right: settings_view::documents::scope_text(self.document_types(cx)),
                })
            }
            ShellView::Settings(_) if self.settings_inventory_page_has_focus(cx) => {
                Some(PageStatus {
                    hints: self.dialog::<InventoryDialog>().map_or_else(
                        || settings_inventory_hints(self.settings_inventory_selected_row(cx)),
                        inventory_ui::dialog_hints,
                    ),
                    right: inventory_view::scope_text(self.inventory(cx)),
                })
            }
            ShellView::Settings(_) if self.settings_categories_page_has_focus(cx) => {
                Some(PageStatus {
                    hints: settings_categories_hints(),
                    right: settings_view::categories::scope_note(self.categories(cx)),
                })
            }
            ShellView::Settings(_) if self.nav.focus() == FocusZone::View => Some(PageStatus {
                hints: self.settings_hints(cx),
                right: self.settings_view.read(cx).selected_section().scope_note(),
            }),
            ShellView::Settings(_) | ShellView::Dashboard | ShellView::Placeholder(_) => None,
        }
    }
}
