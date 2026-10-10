//! The Transactions destination's Entity (ADR-0032). [`TransactionsView`] owns the page's own
//! state: the selected row, the table's scroll, the filters, the search and the filter popover's
//! draft. The rows themselves are read through [`crate::transactions::TransactionsStore`].

use gpui::{Context, UniformListScrollHandle};

use super::header::ChipBounds;
use crate::{
    import::ImportState,
    transactions::{chips::FilterField, filter_form::FilterForm, query::TransactionFilters},
};

/// The Transactions page's own state, owned by its Entity.
pub struct TransactionsState {
    /// The selected row, a position in the filtered list; clamped wherever it is read.
    pub selected: usize,
    /// The table's `uniform_list` scroll state: scrolls the selected row into view and reports the
    /// viewport height for half-page moves.
    pub scroll: UniformListScrollHandle,
    /// What the table is filtered by; starts at the defaults (this year, everything else empty).
    pub filters: TransactionFilters,
    /// The search box's text, separate from the filters.
    pub search: String,
    /// The filter popover's draft, `Some` while it is open (`NavState::mode` is then
    /// `InputMode::Filter`). Kept apart from [`Self::filters`] until **apply**.
    pub filter_form: Option<FilterForm>,
    /// The chip that opened the popover, which it anchors under.
    pub filter_anchor: FilterField,
    /// Where each chip was last painted; the header writes it, the popover reads it.
    pub chip_bounds: ChipBounds,
    /// The stubbed 6e Import "match payees" step, `Some` while it shows in place of the page
    /// (`:import`). Dropped on leaving Transactions.
    pub import: Option<ImportState>,
}

/// The Transactions Entity. Its state is edited through [`Self::edit`], which notifies so the page
/// re-renders.
pub struct TransactionsView {
    state: TransactionsState,
}

impl TransactionsView {
    pub fn new(state: TransactionsState) -> Self {
        Self { state }
    }

    pub fn state(&self) -> &TransactionsState {
        &self.state
    }

    /// Edits the page's state, then asks for a re-render.
    pub fn edit<R>(
        &mut self,
        cx: &mut Context<'_, Self>,
        change: impl FnOnce(&mut TransactionsState) -> R,
    ) -> R {
        let result = change(&mut self.state);
        cx.notify();
        result
    }
}
