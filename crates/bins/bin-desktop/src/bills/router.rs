//! `impl BillsRouter` for the Desktop: where the Bills page's Presenter asks to go. The Plan form
//! and Skip open in today's Dialog host, Pay opens through the Bill Payments module's entry point
//! and a Transaction is shown through the Transactions module's, all through the page's gpui
//! context, which is why the Router is built for each `handle` call. A Notice goes onto the bin's
//! queue (`crate::notices`).

use gpui::{Context, Entity};
use lib_bill_payments::SharedPayments;
use lib_bills::presenter::{BillsPageEvent, PlanFormPresenter, SkipPresenter};
use lib_bills::{BillsOutcome, BillsRouter, EntryId, PayRequest};
use lib_core::{BillPlanId, Clock, DateStyle, TransactionId};
use lib_toast::{Notice, Notify};

use super::ports::DesktopPorts;
use super::view::dialog::BillsDialog;
use super::view::page::BillsPage;
use crate::chrome::dialog_host::open_dialog;
use crate::ledger_store::LedgerStore;
use crate::notices::Queue;
use crate::view::event::ViewEvent;

pub(super) struct PageRouter<'a, 'b> {
    pub ports: DesktopPorts,
    /// What the Pay Dialog writes Pay it directly through.
    pub payments: SharedPayments,
    /// The day and date style the page's Dialogs are opened on.
    pub clock: Clock,
    pub date_style: Option<DateStyle>,
    pub cx: &'a mut Context<'b, BillsPage>,
}

impl PageRouter<'_, '_> {
    /// Hosts `dialog`; its confirmed outcome comes back to the page through
    /// [`BillsPage::dialog_landed`].
    fn host(&mut self, dialog: Option<BillsDialog>) -> bool {
        let Some(dialog) = dialog else {
            return false;
        };
        open_dialog(dialog, BillsPage::dialog_landed, self.cx);
        true
    }
}

impl BillsRouter for PageRouter<'_, '_> {
    fn open_plan_form(&mut self, plan: Option<BillPlanId>) -> bool {
        let ports = self.ports.clone();
        let presenter = match plan {
            None => Some(PlanFormPresenter::add(
                ports,
                self.clock.clone(),
                self.date_style,
            )),
            Some(id) => PlanFormPresenter::edit(ports, id, self.clock.clone(), self.date_style),
        };
        let dialog = presenter.map(|presenter| BillsDialog::plan_form(presenter, &self.ports));
        self.host(dialog)
    }

    fn open_skip(&mut self, entry: EntryId) -> bool {
        let dialog = SkipPresenter::new(self.ports.clone(), entry)
            .map(|presenter| BillsDialog::skip(presenter, &self.ports));
        self.host(dialog)
    }

    /// The Bill Payments module's Pay Dialog; the page redraws from its ports once it lands.
    fn open_pay(&mut self, request: PayRequest) -> bool {
        crate::bill_payments::router::open_pay(
            self.cx,
            &self.ports,
            &self.payments,
            request,
            self.clock.clone(),
            self.date_style,
            BillsPage::pay_landed,
        )
    }

    /// The page closes no Dialog: a Dialog's own Presenter closes it through the host.
    fn close_dialog(&mut self, _outcome: Option<BillsOutcome>) {}

    fn show_transaction(&mut self, id: TransactionId) {
        crate::transactions::router::show(self.cx, id);
    }
}

impl Notify for PageRouter<'_, '_> {
    /// Queues `notice` and asks `Shell` to drain the queue.
    fn notify(&mut self, notice: Notice) {
        Queue.notify(notice);
        self.cx.emit(ViewEvent::NoticesQueued);
    }
}

/// The Bills module's entry point: its page, built on `ports`, which the Shell hosts as the Bills
/// destination. `payments` is what the Pay Dialog it opens writes through, and `clock` reads the
/// calendar day the page dates its rows by.
pub fn bills_page(
    ports: DesktopPorts,
    payments: SharedPayments,
    ledger: Entity<LedgerStore>,
    keymap: lib_viper::keymap::Keymap<BillsPageEvent>,
    clock: Clock,
    cx: &mut gpui::App,
) -> Entity<BillsPage> {
    use gpui::AppContext as _;
    cx.new(|cx| BillsPage::new(ports, payments, ledger, keymap, clock, cx))
}
