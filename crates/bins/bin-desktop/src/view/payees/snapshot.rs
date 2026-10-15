//! What the integration tests read back of Settings' Payees page (ADR-0030), built by the Payees
//! Entity from its own state (ADR-0037).

use gpui::App;

use super::state::PayeesView;
use crate::payees;

/// The Payees page: names A–Z, and the selected row's name.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsPayeesSnapshot {
    pub payee_names: Vec<String>,
    pub selected_payee: Option<String>,
}

impl PayeesView {
    #[doc(hidden)]
    pub fn snapshot(&self, cx: &App) -> SettingsPayeesSnapshot {
        let rows = self.payees(cx);
        SettingsPayeesSnapshot {
            payee_names: payees::sorted_by_name(rows)
                .into_iter()
                .map(|payee| payee.name.clone())
                .collect(),
            selected_payee: self
                .selected_id(cx)
                .and_then(|id| payees::get(rows, id))
                .map(|payee| payee.name.clone()),
        }
    }
}
