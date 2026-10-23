//! `ActiveView` -- the one thing `Shell` dispatches on to decide which View owns the keyboard,
//! the page status and the main pane: `lib-shell`'s, over the Desktop's Settings pages
//! (ADR-0016). The mode label it names is a Message, so it stays here.

use crate::settings::SettingsPage;

pub type ActiveView = lib_shell::ActiveView<SettingsPage>;

/// The status line's mode label in `Normal` mode, where the View on show names its own: the
/// Import step's `IMPORT`.
pub fn mode_label(view: ActiveView) -> Option<String> {
    match view {
        ActiveView::Import => Some(crate::msg::desktop_mode_import()),
        ActiveView::Dashboard
        | ActiveView::Transactions
        | ActiveView::Documents
        | ActiveView::Bills
        | ActiveView::Budgets
        | ActiveView::Settings(_)
        | ActiveView::Placeholder(_) => None,
    }
}
