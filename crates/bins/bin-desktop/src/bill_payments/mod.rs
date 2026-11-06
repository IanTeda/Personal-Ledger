//! The Bill Payments module's Desktop half (`docs/development/viper-architecture.md`): the Pay
//! Dialog's View and the entry point another module's Router opens it through. Every rule, every
//! piece of text and the Presenter are `lib-bill-payments`'; this folder turns gpui keys and clicks
//! into Presenter events and draws the view model.
//!
//! - `view/dialog/pay.rs` holds the Pay Dialog's Presenter in an Entity in the Dialog host and draws it.
//! - `router.rs` is the module's entry point ([`router::open_pay`]), which the Bills Router calls.
//!
//! Pay it directly writes through [`payments`], `lib-database`'s adapter over the Transactions
//! store the Open Ledger holds and the Bills store the Bills ports hold, so its Transaction and entry
//! join the one pending batch and land in one commit.

pub mod router;
pub(crate) mod view;

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{App, Entity};
use lib_bill_payments::SharedPayments;
use lib_database::{BillPaymentsAdapter, SharedTransactionsStore};

use crate::announcer::{Announcer, Domain};
use crate::bills::DesktopPorts;

pub use view::dialog::pay::PayDialog;

/// The payments port over `transactions` and the Bills store `bills` writes.
pub fn payments(transactions: SharedTransactionsStore, bills: &DesktopPorts) -> SharedPayments {
    Rc::new(RefCell::new(BillPaymentsAdapter::new(
        transactions,
        bills.borrow().store(),
    )))
}

/// Announces the Transactions and the Bills after a confirm, so the Transactions View, the
/// Dashboard and the Bills page see the payment at once (#1028). The confirm's
/// input commits it.
pub(crate) fn announce(announcer: &Entity<Announcer>, cx: &mut App) {
    announcer.update(cx, |announcer, cx| {
        announcer.announce(Domain::Transactions, cx);
    });
    crate::bills::ports::announce(announcer, cx);
}
