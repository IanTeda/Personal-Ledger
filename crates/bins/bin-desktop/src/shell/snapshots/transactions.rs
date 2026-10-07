//! What the integration tests read back of the Transactions page, kept apart from `shell.rs` so
//! the snapshot shape and its accessor sit together.

use crate::shell::Shell;
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
    pub fn transactions_snapshot(&self) -> TransactionsSnapshot {
        let ledger = self.transactions_ledger();
        let visible = transactions::query::query(
            &ledger,
            &self.transactions,
            &self.transactions_filters,
            &self.transactions_search,
        );
        let rows = transactions::rows::build_rows(&visible, &ledger, &self.transactions_prefs());
        let chips = transactions::chips::chips(
            &self.transactions_filters,
            &ledger,
            self.today,
            self.settings_date_style,
        );
        TransactionsSnapshot {
            visible: rows.len(),
            selected: transactions::rows::clamp_selection(self.transactions_selected, rows.len()),
            payees: rows.into_iter().map(|row| row.payee).collect(),
            search: self.transactions_search.clone(),
            chips: chips
                .into_iter()
                .map(|chip| ChipSnapshot {
                    label: chip.label,
                    active: chip.active,
                })
                .collect(),
            filters_changed: !self.transactions_filters.is_default(self.today),
            filter_open: self.transactions_filter_form.is_some(),
            filter_focus: self
                .transactions_filter_form
                .as_ref()
                .map(|form| format!("{:?}", form.focused)),
            draft_status: self
                .transactions_filter_form
                .as_ref()
                .map(|form| format!("{:?}", form.status)),
            account_list_open: self
                .transactions_filter_form
                .as_ref()
                .is_some_and(|form| form.account.is_open()),
        }
    }
}
