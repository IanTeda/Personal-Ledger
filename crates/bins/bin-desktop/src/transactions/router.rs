//! The Transactions module's entry points: [`transactions_page`], the page, and, for other
//! modules' Routers, as `bill_payments::router::open_pay` is for the Pay Dialog, [`show`] and
//! [`show_filtered`], which hand the page what to show and ask the Shell to make it the
//! destination. Every hand-off into the page calls one of them, from Bills, Documents, Accounts,
//! Payees, Tags and the Budgets detail, and the Import's commit lands through [`first_row`]. The
//! page's Presenter asks only for a Notice, which the generic page Router queues.
//!
//! The hand-offs reach the page through a gpui global holding it weakly, the bin's only one, not a
//! `ViewEvent::Reveal` routed by `Root` as other Views' hand-offs are (#1127):
//!
//! - A hand-off runs synchronously inside the sending input and its one commit: the sender's
//!   Router calls it while its Presenter handles, so the page has shown what was handed it before
//!   that input ends. An emitted event is handled later, by `Root`, outside it.
//! - [`TransactionFilters`] isn't `Copy`, which `Reveal` and `PageCue` are.
//! - [`first_row`] doesn't navigate: the Import's commit lands on the page already the
//!   destination.

use gpui::{App, AppContext as _, Entity, Global, WeakEntity};
use lib_core::configuration::KeyBindingConfig;
use lib_core::{Clock, TransactionId};
use lib_list_cursor::Step;
use lib_transactions::SharedPorts;
use lib_transactions::presenter::{TransactionsPageEvent, TransactionsPagePresenter};
use lib_transactions::query::TransactionFilters;

use super::TransactionsPage;
use crate::announcer::Announcer;
use crate::navigation::nav::Noun;
use crate::shell::view::module_page::ModulePage;
use crate::view::event::ViewEvent;

/// The Transactions module's entry point: the page on `ports`, filtered to this year by `clock`'s
/// today, under `config`'s keys.
pub fn transactions_page(
    ports: SharedPorts,
    announcer: &Entity<Announcer>,
    config: &KeyBindingConfig,
    clock: Clock,
    cx: &mut App,
) -> Entity<TransactionsPage> {
    let presenter = TransactionsPagePresenter::new(ports, clock, config);
    cx.new(|cx| ModulePage::new(presenter, (), announcer, cx))
}

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
    hand_off(cx, TransactionsPageEvent::RevealTransaction(id), true);
}

/// Shows the Transactions page filtered to `filters`, from its first row, with no Search or draft
/// left over.
pub fn show_filtered(cx: &mut App, filters: TransactionFilters) {
    hand_off(cx, TransactionsPageEvent::ShowFiltered(filters), true);
}

/// Puts the Transactions page back on its first row, where an Import commit lands. The page is
/// already the destination, the Noun staying Transactions under the Import.
pub fn first_row(cx: &mut App) {
    hand_off(cx, TransactionsPageEvent::Move(Step::First), false);
}

/// Sends `event` to the installed page, then, when `navigate`, asks the Shell to show it. Another
/// module's input is already being sent, so the hand-off joins it and its one commit; one from a
/// View that isn't a module yet is an input of its own. Without an installed page (a part built
/// with no Shell) there is nowhere to show it.
fn hand_off(cx: &mut App, event: TransactionsPageEvent, navigate: bool) {
    let Some(page) = cx.try_global::<Page>().and_then(|page| page.0.upgrade()) else {
        return;
    };
    page.update(cx, |page, cx| {
        page.send(event, cx);
        if navigate {
            cx.emit(ViewEvent::Navigate(Noun::Transactions));
        }
    });
}
