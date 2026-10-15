//! The Transactions destination's cross-cutting wiring: the rows other pages read, `:import`, and
//! the "View transactions" links that land on the page filtered. The page itself renders and
//! handles its own input in its Entity (`view::transactions`, ADR-0037).

use super::Shell;

use crate::{
    navigation::nav::{FocusZone, Noun},
    transactions::{Transaction, query::TransactionFilters},
};

use gpui::App;

impl Shell {
    /// The Transactions, read through their store (ADR-0032).
    pub(super) fn transactions<'a>(&self, cx: &'a App) -> &'a [Transaction] {
        self.ledger.read(cx).ledger().transactions()
    }

    /// `:import`: opens the stubbed 6e step on the seeded statement, in place of the Transactions
    /// page (#284).
    pub(super) fn open_import(&mut self, cx: &mut App) {
        self.nav.set_noun(Noun::Transactions);
        self.nav.set_focus(FocusZone::View);
        self.reset_view_scroll(cx);
        self.views
            .transactions
            .update(cx, |view, cx| view.open_import(cx));
    }

    /// The Transactions page filtered to `filters` from its first row: the "View transactions"
    /// links of Accounts, Payees, Tags and the Budgets detail.
    pub(super) fn open_transactions_filtered(&mut self, filters: TransactionFilters, cx: &mut App) {
        self.views
            .transactions
            .update(cx, |view, cx| view.show_filtered(filters, cx));
        self.nav.set_noun(Noun::Transactions);
        self.reset_view_scroll(cx);
    }

    /// "View transactions" on the Payees page: Transactions filtered to exactly this Payee's id, not
    /// a name substring that would over-match ("BP").
    pub(super) fn open_payee_transactions(&mut self, id: u32, cx: &mut App) {
        self.open_transactions_filtered(TransactionFilters::for_payee(self.today, id), cx);
    }

    /// "View transactions" on the Tags page: Transactions filtered to exactly this Tag's id, not a
    /// name substring that would over-match ("trip").
    pub(super) fn open_tag_transactions(&mut self, id: u32, cx: &mut App) {
        self.open_transactions_filtered(TransactionFilters::for_tag(self.today, id), cx);
    }
}
