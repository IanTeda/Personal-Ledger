//! The Transactions module's entry point for other modules' Routers, as
//! `bill_payments::router::open_pay` is for the Pay Dialog: [`show`] and [`show_filtered`] hand
//! the page what to show and ask the Shell to make it the destination. Every hand-off into the
//! page calls one of them, from Bills, Documents, Accounts, Payees, Tags and the Budgets detail.

use gpui::{App, Entity, Global, WeakEntity};
use lib_core::TransactionId;
use lib_transactions::query::TransactionFilters;

use super::TransactionsPage;

/// The page the entry point reaches, installed once by `shell::router::build`'s `Root`.
struct Page(WeakEntity<TransactionsPage>);

impl Global for Page {}

/// Makes `page` the one the entry point hands off to.
pub(crate) fn install(page: &Entity<TransactionsPage>, cx: &mut App) {
    cx.set_global(Page(page.downgrade()));
}

/// Shows Transaction `id` on the Transactions page, selected, with the date range widened to reach
/// it. A Transaction that has gone leaves the page as it was.
pub fn show(cx: &mut App, id: TransactionId) {
    hand_off(cx, |page, cx| page.show(id, cx));
}

/// Shows the Transactions page filtered to `filters`, from its first row.
pub fn show_filtered(cx: &mut App, filters: TransactionFilters) {
    hand_off(cx, |page, cx| page.show_filtered(filters, cx));
}

/// Runs `hand_off` on the installed page. Without one (a part built with no Shell) there is
/// nowhere to show it.
fn hand_off(
    cx: &mut App,
    hand_off: impl FnOnce(&mut TransactionsPage, &mut gpui::Context<'_, TransactionsPage>),
) {
    let Some(page) = cx.try_global::<Page>().and_then(|page| page.0.upgrade()) else {
        return;
    };
    page.update(cx, hand_off);
}
