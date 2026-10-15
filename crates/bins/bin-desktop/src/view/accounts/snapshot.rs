//! What the integration tests read back of Settings' Accounts page (ADR-0030), built by the
//! Accounts Entity from its own state (ADR-0037).

use gpui::App;
use lib_accounts::display_order;

use super::state::AccountsView;

/// The Accounts page: names in the order the page lists them, and the selected row's name.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsAccountsSnapshot {
    pub account_names: Vec<String>,
    pub selected_account: Option<String>,
}

impl AccountsView {
    #[doc(hidden)]
    pub fn snapshot(&self, cx: &App) -> SettingsAccountsSnapshot {
        let accounts = self.accounts(cx);
        SettingsAccountsSnapshot {
            account_names: display_order(accounts)
                .into_iter()
                .filter_map(|index| accounts.get(index))
                .map(|account| account.name.clone())
                .collect(),
            selected_account: self
                .selected_index(cx)
                .and_then(|index| accounts.get(index))
                .map(|account| account.name.clone()),
        }
    }
}
