//! The Transactions destination's cross-cutting wiring: the rows other pages read, `:import`, and
//! the "View transactions" links that land on the page filtered. The page itself renders and
//! handles its own input in its Entity (`view::transactions`, ADR-0037).

use super::Root;
use lib_core::{PayeeId, TagId};
use lib_shell::NavInput;

use crate::{
    navigation::nav::{FocusZone, Noun},
    transactions::{Transaction, query::TransactionFilters},
};

use gpui::App;

impl Root {
    /// The Transactions, read through their store (ADR-0032).
    pub(super) fn transactions<'a>(&self, cx: &'a App) -> &'a [Transaction] {
        self.ledger.read(cx).ledger().transactions()
    }

    /// `:import`: opens the stubbed 6e step on the seeded statement, the Import View shown in place
    /// of the Transactions list (#284, #924).
    pub(super) fn open_import(&mut self, cx: &mut App) {
        self.navigate(NavInput::Show(Noun::Transactions));
        self.navigate(NavInput::Focus(FocusZone::View));
        self.reset_view_scroll(cx);
        self.views.import.update(cx, |view, cx| view.open(cx));
    }

    /// The Transactions page filtered to `filters` from its first row: the "View transactions"
    /// links of Accounts, Payees, Tags and the Budgets detail.
    pub(super) fn open_transactions_filtered(&mut self, filters: TransactionFilters, cx: &mut App) {
        self.views
            .transactions
            .update(cx, |view, cx| view.show_filtered(filters, cx));
        self.navigate(NavInput::Show(Noun::Transactions));
        self.reset_view_scroll(cx);
    }

    /// "View transactions" on the Payees page: Transactions filtered to exactly this Payee's id, not
    /// a name substring that would over-match ("BP").
    pub(super) fn open_payee_transactions(&mut self, id: PayeeId, cx: &mut App) {
        self.open_transactions_filtered(TransactionFilters::for_payee(self.today, id), cx);
    }

    /// "View transactions" on the Tags page: Transactions filtered to exactly this Tag's id, not a
    /// name substring that would over-match ("trip").
    pub(super) fn open_tag_transactions(&mut self, id: TagId, cx: &mut App) {
        self.open_transactions_filtered(TransactionFilters::for_tag(self.today, id), cx);
    }
}
