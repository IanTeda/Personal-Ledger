//! `impl PayeesRouter` for the Desktop: where the Payees page's Presenter asks to go. A Dialog
//! opens as an Entity in the Dialog host through the page's gpui context, which is why the Router
//! is built for each `handle` call, and a Notice goes onto the bin's queue (`crate::notices`),
//! which the input's `send` drains.

use gpui::Context;
use lib_payees::presenter::PayeesPageEvent;
use lib_payees::{PayeesDialog as Which, PayeesOutcome, PayeesRouter};
use lib_toast::{Notice, Notify};
use lib_transactions::query::TransactionFilters;

use super::ports::DesktopPorts;
use super::view::dialog::PayeesDialog;
use super::view::page::PayeesPage;
use crate::ledger_store::LedgerStore;
use crate::notices::Queue;
use crate::shell::view::dialog_entity::host_dialog;

pub(super) struct PageRouter<'a, 'b> {
    pub ports: DesktopPorts,
    pub ledger: gpui::Entity<LedgerStore>,
    pub clock: lib_core::Clock,
    pub cx: &'a mut Context<'b, PayeesPage>,
}

impl PayeesRouter for PageRouter<'_, '_> {
    /// Builds `dialog`'s Presenter on the shared ports and hosts its View; its confirmed outcome
    /// comes back to the page through [`PayeesPage::dialog_landed`].
    fn open_dialog(&mut self, dialog: Which) -> bool {
        let page = self.cx.weak_entity();
        let landed = Box::new(move |outcome, cx: &mut gpui::App| {
            // A page that has gone has nothing to land on.
            let _ = page.update(cx, |page, cx| page.dialog_landed(outcome, cx));
        });
        let Some(dialog) = PayeesDialog::open(dialog, &self.ports, &self.ledger, landed) else {
            return false;
        };
        host_dialog(self.cx, |_| dialog);
        true
    }

    /// Shows the Payee's Transactions through the Transactions entry point: the page
    /// filtered to it, from its first row.
    fn open_transactions(&mut self, id: lib_core::PayeeId) {
        let filters = TransactionFilters::for_payee(self.clock.today(), id);
        crate::transactions::router::show_filtered(self.cx, filters);
    }

    /// The page closes no Dialog: a Dialog's own Presenter closes it through the host.
    fn close_dialog(&mut self, _outcome: Option<PayeesOutcome>) {}
}

impl Notify for PageRouter<'_, '_> {
    /// Queues `notice`, for the input's `send` to drain into the Shell.
    fn notify(&mut self, notice: Notice) {
        Queue.notify(notice);
    }
}

/// The Payees module's entry point: its Settings page, built on `ports`, which the Settings View
/// hosts as any other page.
pub fn payees_page(
    ports: DesktopPorts,
    ledger: gpui::Entity<LedgerStore>,
    clock: lib_core::Clock,
    keymap: lib_viper::keymap::Keymap<PayeesPageEvent>,
    cx: &mut gpui::App,
) -> gpui::Entity<PayeesPage> {
    use gpui::AppContext as _;
    cx.new(|cx| PayeesPage::new(ports, ledger, clock, keymap, cx))
}
