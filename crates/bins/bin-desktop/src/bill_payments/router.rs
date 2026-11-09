//! The Bill Payments module's entry point for other modules' Routers ([`open_pay`]). The Pay Dialog
//! is the Pay Presenter in a `ModuleDialog`, closing through the host's `HostRouter`.

use gpui::{App, Context, EventEmitter};
use lib_bill_payments::presenter::PayPresenter;
use lib_bill_payments::{PayOutcome, SharedPayments};
use lib_bills::PayRequest;
use lib_core::{Clock, DateStyle};
use lib_viper::dialog::DialogKeys;

use crate::shell::view::dialog_entity::ModuleDialog;
use crate::view::event::ViewEvent;

/// What the Pay Dialog reads and writes through: the Bills ports its Match panel and wording read,
/// and the port Pay it directly writes through.
pub struct PayStores<'a> {
    pub bills: &'a lib_bills::SharedPorts,
    pub payments: &'a SharedPayments,
}

/// What the Pay Dialog opens on beside its entry: the day and date style its Date field defaults
/// to and parses by, and the Dialog keys its hints show.
pub struct PayOpening {
    pub clock: Clock,
    pub date_style: Option<DateStyle>,
    pub keys: DialogKeys,
}

/// Opens the Pay Dialog on `request`'s entry over page `P`; a confirm hands its outcome to
/// `landed`, mapped into the page's own, so the page redraws from its ports. Returns whether it
/// opened: an entry whose Plan has gone has no Dialog.
pub fn open_pay<P: EventEmitter<ViewEvent> + 'static>(
    cx: &mut Context<'_, P>,
    stores: PayStores<'_>,
    request: PayRequest,
    opening: PayOpening,
    landed: impl Fn(PayOutcome, &mut App) + 'static,
) -> bool {
    let PayStores { bills, payments } = stores;
    let PayOpening {
        clock,
        date_style,
        keys,
    } = opening;
    let Some(presenter) = PayPresenter::new(
        bills.clone(),
        payments.clone(),
        request,
        clock,
        date_style,
        keys,
    ) else {
        return false;
    };
    ModuleDialog::open(cx, presenter, landed);
    true
}
