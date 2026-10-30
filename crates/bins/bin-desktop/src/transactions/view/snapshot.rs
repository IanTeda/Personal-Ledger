//! What the integration tests read back of the Transactions page (ADR-0030), read from the view
//! model its Presenter builds.

use gpui::App;
use lib_transactions::presenter::FilterField;

use super::page::TransactionsPage;

/// One filter chip as the header shows it.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChipSnapshot {
    pub label: String,
    /// Changed from the default: drawn as an accent chip with a `✕`.
    pub active: bool,
}

/// The Transactions page's state: plain values, so the private query types stay private.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionsSnapshot {
    /// How many rows the filters and search leave visible.
    pub visible: usize,
    /// The selected position in the visible rows, already clamped.
    pub selected: usize,
    /// The PAYEE cell of every visible row, in table order.
    pub payees: Vec<String>,
    pub search: String,
    pub chips: Vec<ChipSnapshot>,
    /// The `clear filters` link is showing (a filter differs from the default).
    pub filters_changed: bool,
    pub filter_open: bool,
    /// The focused popover field, e.g. `Payee`; `None` while the popover is closed.
    pub filter_focus: Option<String>,
    /// The Status control's draft in the open popover.
    pub draft_status: Option<String>,
    /// The popover's Account select is open (its option list showing).
    pub account_list_open: bool,
}

impl TransactionsPage {
    /// The page for a test, read from the view model it draws.
    #[doc(hidden)]
    pub fn snapshot(&self, _cx: &App) -> TransactionsSnapshot {
        let view = self.view_model();
        let filter = view.filter.as_ref();
        TransactionsSnapshot {
            visible: view.rows.len(),
            selected: view.selected,
            payees: view.rows.iter().map(|row| row.payee.clone()).collect(),
            search: view.search.clone(),
            chips: view
                .chips
                .iter()
                .map(|chip| ChipSnapshot {
                    label: chip.label.clone(),
                    active: chip.active,
                })
                .collect(),
            filters_changed: view.filters_changed,
            filter_open: filter.is_some(),
            filter_focus: filter.map(|view| format!("{:?}", view.focused)),
            draft_status: filter
                .and_then(|view| view.status.chosen)
                .map(|status| format!("{status:?}")),
            account_list_open: self
                .filter_list
                .as_ref()
                .is_some_and(|(field, list)| *field == FilterField::Account && list.is_open()),
        }
    }
}
