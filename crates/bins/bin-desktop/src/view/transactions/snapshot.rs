//! What the integration tests read back of the Transactions page and its Import step (ADR-0030),
//! built by the Transactions Entity from its own state (ADR-0037).

use gpui::App;

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

/// One Import row: the raw statement text and its status.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportRowSnapshot {
    pub raw: String,
    /// `RuleMatch`, `NewPayee`, `Matched` or `NeedsReview`.
    pub status: String,
}

/// The Import step's state.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportSnapshot {
    pub rows: Vec<ImportRowSnapshot>,
    pub selected: usize,
    pub remember: bool,
    /// `Payee` or `Category` while a row's select is open.
    pub open_select: Option<String>,
    pub can_continue: bool,
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
        let chips =
            transactions::chips::chips(&state.filters, &ledger, self.today, self.date_style);
        let form = state.filter_form.as_ref();
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
            filter_open: form.is_some(),
            filter_focus: form.map(|form| format!("{:?}", form.focused)),
            draft_status: form.map(|form| format!("{:?}", form.status)),
            account_list_open: form.is_some_and(|form| form.account.is_open()),
        }
    }

    /// The Import step for a test; `None` while it is not showing.
    #[doc(hidden)]
    pub fn import_snapshot(&self) -> Option<ImportSnapshot> {
        let state = self.state.import.as_ref()?;
        Some(ImportSnapshot {
            rows: state
                .rows
                .iter()
                .map(|row| ImportRowSnapshot {
                    raw: row.raw.clone(),
                    status: format!("{:?}", row.status()),
                })
                .collect(),
            selected: state.selected,
            remember: state.remember,
            open_select: state
                .open_select
                .as_ref()
                .map(|(_, select, _)| format!("{select:?}")),
            can_continue: state.can_continue(),
        })
    }
}
