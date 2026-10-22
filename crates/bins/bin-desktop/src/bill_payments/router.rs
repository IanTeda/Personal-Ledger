//! The Bill Payments module's entry point for other modules' Routers ([`open_pay`]), and
//! `impl BillPaymentsRouter` for the Pay Dialog. The Dialog's Presenter only ever asks to close,
//! so its Router records that for the host's confirm to read.

use chrono::NaiveDate;
use gpui::{Context, EventEmitter};
use lib_bill_payments::presenter::PayPresenter;
use lib_bill_payments::{BillPaymentsRouter, PayOutcome, SharedPayments};
use lib_bills::PayRequest;
use lib_core::DateStyle;

use super::PayDialog;
use crate::bills::DesktopPorts;
use crate::chrome::dialog_host::open_dialog;
use crate::view::event::ViewEvent;

/// Where the Pay Dialog's Presenter asks to go: only closing, which the host does.
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

/// Opens the Pay Dialog on `request`'s entry over page `P`, whose `landed` runs with the outcome
/// once a confirm lands, so the page redraws from its ports. `bills` are the Bills ports the
/// Dialog's Match panel and wording read, `payments` the port Pay it directly writes through, and
/// `today` and `date_style` what its Date field defaults to and parses by. Returns whether it
/// opened: an entry whose Plan has gone has no Dialog.
pub fn open_pay<P: EventEmitter<ViewEvent> + 'static>(
    cx: &mut Context<'_, P>,
    bills: &DesktopPorts,
    payments: &SharedPayments,
    request: PayRequest,
    today: NaiveDate,
    date_style: Option<DateStyle>,
    landed: impl Fn(&mut P, PayOutcome, &mut Context<'_, P>) + 'static,
) -> bool {
    let Some(presenter) =
        PayPresenter::new(bills.clone(), payments.clone(), request, today, date_style)
    else {
        return false;
    };
    open_dialog(PayDialog::new(presenter, bills), landed, cx);
    true
}
