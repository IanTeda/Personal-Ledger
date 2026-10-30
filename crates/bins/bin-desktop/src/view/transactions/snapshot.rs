//! What the integration tests read back of the Transactions page (ADR-0030), built by the
//! Transactions Entity from its own state (ADR-0037).

use gpui::App;
use lib_transactions::presenter::{FilterField, FilterPresenter};

use super::state::TransactionsView;
use crate::transactions;

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

impl TransactionsView {
    /// The page for a test, built the way the render builds its rows.
    #[doc(hidden)]
    pub fn snapshot(&self, cx: &App) -> TransactionsSnapshot {
        let state = &self.state;
        let ledger = self.ledger(cx);
        let visible = transactions::query::query(
            &ledger,
            self.transactions(cx),
            &state.filters,
            &state.search,
        );
        let rows = transactions::rows::build_rows(&visible, &ledger, &self.prefs());
        let chips = FilterPresenter::chips(&ledger, &state.filters, self.today, self.date_style);
        let filter = self.filter_view();
        TransactionsSnapshot {
            visible: rows.len(),
            selected: transactions::rows::clamp_selection(state.selected, rows.len()),
            payees: rows.into_iter().map(|row| row.payee).collect(),
            search: state.search.clone(),
            chips: chips
                .into_iter()
                .map(|chip| ChipSnapshot {
                    label: chip.label,
                    active: chip.active,
                })
                .collect(),
            filters_changed: !state.filters.is_default(self.today),
            filter_open: filter.is_some(),
            filter_focus: filter.as_ref().map(|view| format!("{:?}", view.focused)),
            draft_status: filter
                .as_ref()
                .and_then(|view| view.status.chosen)
                .map(|status| format!("{status:?}")),
            account_list_open: state
                .filter_list
                .as_ref()
                .is_some_and(|(field, list)| *field == FilterField::Account && list.is_open()),
        }
    }
}
