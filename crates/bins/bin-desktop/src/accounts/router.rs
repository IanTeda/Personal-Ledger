//! `impl AccountsRouter` for the Desktop: where the Accounts page's Presenter asks to go. The Add
//! and Edit Dialogs open as an Entity in the Dialog host through the page's gpui context, which is
//! why the Router is built for each `handle` call, and a Notice goes onto the bin's queue
//! (`crate::notices`), which the input's `send` drains.

use gpui::Context;

use lib_accounts::{AccountsDialog as Which, AccountsOutcome, AccountsRouter};
use lib_core::AccountId;
use lib_toast::{Notice, Notify};

use super::ports::DesktopPorts;
use super::view::dialog::AccountsDialog;
use super::view::page::AccountsPage;
use crate::account_deletion::DeletionStores;
use crate::ledger_store::LedgerStore;
use crate::notices::Queue;
use crate::shell::view::dialog_entity::host_dialog;
use crate::view::event::{Reveal, ViewEvent};

pub(super) struct PageRouter<'a, 'b> {
    pub ports: DesktopPorts,
    pub ledger: gpui::Entity<LedgerStore>,
    pub deletion: DeletionStores,
    pub clock: lib_core::Clock,
    pub cx: &'a mut Context<'b, AccountsPage>,
}

impl AccountsRouter for PageRouter<'_, '_> {
    /// Builds `dialog`'s Presenter on the shared ports and hosts its View; its confirmed outcome
    /// comes back to the page through [`AccountsPage::dialog_landed`].
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let page = self.cx.weak_entity();
        let landed = Box::new(move |outcome, cx: &mut gpui::App| {
            // A page that has gone has nothing to land on.
            let _ = page.update(cx, |page, cx| page.dialog_landed(outcome, cx));
        });
        let Some(dialog) = AccountsDialog::open(
            dialog,
            &self.ports,
            &self.ledger,
            self.clock.today(),
            landed,
        ) else {
            return false;
        };
        host_dialog(self.cx, |_| dialog);
        true
    }

    /// The Account Deletion module's Delete Dialog, through its entry point; a confirmed delete
    /// comes back to the page, which selects the row that slid into the deleted one's place.
    fn open_delete(&mut self, id: AccountId) -> bool {
        crate::account_deletion::router::open_delete(
            self.cx,
            &self.deletion,
            &self.ledger,
            id,
            |page: &mut AccountsPage, _deleted, cx| page.deleted(cx),
        )
    }

    /// Asks `Root` for the Account's ledger: the Transactions filtered to it, from their first
    /// row. The selection is untouched, so returning to Accounts finds the same row selected.
    fn open_ledger(&mut self, id: AccountId) {
        self.cx
            .emit(ViewEvent::Reveal(Reveal::AccountTransactions(id)));
    }

    /// The page closes no Dialog: the Dialog's own Presenter closes it through the host.
    fn close_dialog(&mut self, _outcome: Option<AccountsOutcome>) {}
}

impl Notify for PageRouter<'_, '_> {
    /// Queues `notice`, for the input's `send` to drain into the Shell.
    fn notify(&mut self, notice: Notice) {
        Queue.notify(notice);
    }
}

/// The Accounts module's entry point: its Settings page, built on `ports`, which the Settings View
/// hosts as any other page.
pub fn accounts_page(
    setup: super::AccountsSetup,
    ledger: gpui::Entity<LedgerStore>,
    clock: lib_core::Clock,
    cx: &mut gpui::App,
) -> gpui::Entity<AccountsPage> {
    use gpui::AppContext as _;
    let super::AccountsSetup {
        ports,
        deletion,
        keymap,
    } = setup;
    cx.new(|cx| AccountsPage::new(ports, deletion, ledger, clock, keymap, cx))
}
