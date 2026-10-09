//! What the integration tests read back of the Transactions page, kept apart from `shell.rs` so
//! the snapshot shape and its accessor sit together.

use crate::app::Shell;
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

impl Shell {
    /// The Transactions page for a test, built the way the render builds its rows.
    #[doc(hidden)]
    pub fn transactions_snapshot(&self, cx: &gpui::App) -> TransactionsSnapshot {
        // The rows and the page state come from their Entities, the way the render reads them;
        // only the reference data and the Display preferences are still `Shell` fields.
        let transactions = self.transactions_store.read(cx).transactions();
        let state = self.transactions_view.read(cx).state();
        let ledger = self.transactions_ledger(cx);
        let visible =
            transactions::query::query(&ledger, transactions, &state.filters, &state.search);
        let rows = transactions::rows::build_rows(&visible, &ledger, &self.transactions_prefs());
        let chips = transactions::chips::chips(
            &state.filters,
            &ledger,
            self.today,
            self.settings_date_style,
        );
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
            filter_open: state.filter_form.is_some(),
            filter_focus: self
                .transactions_state(cx)
                .filter_form
                .as_ref()
                .map(|form| format!("{:?}", form.focused)),
            draft_status: self
                .transactions_state(cx)
                .filter_form
                .as_ref()
                .map(|form| format!("{:?}", form.status)),
            account_list_open: self
                .transactions_state(cx)
                .filter_form
                .as_ref()
                .is_some_and(|form| form.account.is_open()),
        }
    }
}
