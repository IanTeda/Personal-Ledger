//! The Bill Payments module's entry point for other modules' Routers ([`open_pay`]), and
//! `impl BillPaymentsRouter` for the Pay Dialog. Its Router records a close for the Dialog to read, and puts a Notice onto the bin's queue (`crate::notices`).

use gpui::{Context, EventEmitter};
use lib_bill_payments::presenter::PayPresenter;
use lib_bill_payments::{BillPaymentsRouter, PayOutcome, SharedPayments};
use lib_bills::PayRequest;
use lib_core::{Clock, DateStyle};
use lib_toast::{Notice, Notify};

use super::PayDialog;
use crate::bills::DesktopPorts;
use crate::ledger_store::LedgerStore;
use crate::shell::view::dialog_entity::host_dialog;
use crate::view::event::ViewEvent;

/// Where the Pay Dialog's Presenter asks to go: closing, which the Dialog reads, and Notices, which
/// `Root` drains once the key or click returns.
#[derive(Default)]
pub(super) struct DialogRouter {
    /// `Some` once the Presenter closed the Dialog, with what its confirm did.
    pub closed: Option<Option<PayOutcome>>,
}

impl BillPaymentsRouter for DialogRouter {
    fn close_dialog(&mut self, outcome: Option<PayOutcome>) {
        self.closed = Some(outcome);
    }
}

impl Notify for DialogRouter {
    fn notify(&mut self, notice: Notice) {
        crate::notices::Queue.notify(notice);
    }
}

/// What the Pay Dialog reads and writes through: the Bills ports its Match panel and wording read,
/// the port Pay it directly writes through, and the Ledger read copy a confirm reads and brings in
/// line.
pub struct PayStores<'a> {
    pub bills: &'a DesktopPorts,
    pub payments: &'a SharedPayments,
    pub ledger: &'a gpui::Entity<LedgerStore>,
}

/// Opens the Pay Dialog on `request`'s entry over page `P`, whose `landed` runs with the outcome
/// once a confirm lands, so the page redraws from its ports. `clock` and `date_style` are what its
/// Date field defaults to and parses by. Returns whether it opened: an entry whose Plan has gone
/// has no Dialog.
pub fn open_pay<P: EventEmitter<ViewEvent> + 'static>(
    cx: &mut Context<'_, P>,
    stores: PayStores<'_>,
    request: PayRequest,
    clock: Clock,
    date_style: Option<DateStyle>,
    landed: impl Fn(&mut P, PayOutcome, &mut Context<'_, P>) + 'static,
) -> bool {
    let PayStores {
        bills,
        payments,
        ledger,
    } = stores;
    let Some(presenter) =
        PayPresenter::new(bills.clone(), payments.clone(), request, clock, date_style)
    else {
        return false;
    };
    let page = cx.weak_entity();
    let landed = Box::new(move |outcome, cx: &mut gpui::App| {
        // A page that has gone has nothing to land on.
        let _ = page.update(cx, |page, cx| landed(page, outcome, cx));
    });
    let dialog = PayDialog::new(presenter, bills, ledger, landed);
    host_dialog(cx, |_| dialog);
    true
}
