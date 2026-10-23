//! The Bill Payments module's Desktop half (`docs/development/viper-architecture.md`): the Pay
//! Dialog's View and the entry point another module's Router opens it through. Every rule, every
//! piece of text and the Presenter are `lib-bill-payments`'; this folder turns gpui keys and clicks
//! into Presenter events and draws the view model.
//!
//! - `view/dialog/pay.rs` hosts the Pay Dialog's Presenter in the Dialog host and draws it.
//! - `router.rs` is the module's entry point ([`router::open_pay`]), which the Bills Router calls.
//!
//! Pay it directly writes through [`payments`], `lib-database`'s adapter over the Transactions
//! store `LedgerStore` holds and the Bills store the Bills ports hold, so its Transaction and entry
//! join the one pending batch and land in one commit.

pub mod router;
pub(crate) mod view;

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{App, Entity};
use lib_bill_payments::SharedPayments;
use lib_database::{BillPaymentsAdapter, SharedTransactionsStore};

use crate::bills::DesktopPorts;
use crate::ledger_store::LedgerStore;

pub use view::dialog::pay::PayDialog;

/// The payments port over `transactions` and the Bills store `bills` writes.
pub fn payments(transactions: SharedTransactionsStore, bills: &DesktopPorts) -> SharedPayments {
    Rc::new(RefCell::new(BillPaymentsAdapter::new(
        transactions,
        bills.borrow().store(),
    )))
}

/// Brings the Ledger's Transactions and Bills read copies in line after a confirm, so the
/// Transactions View, the Dashboard and the Bills page see the payment at once, and copies the
/// reads back into the Bills ports for the next event. The confirm's input commits it.
pub(crate) fn refresh(bills: &DesktopPorts, ledger: &Entity<LedgerStore>, cx: &mut App) {
    ledger.update(cx, |store, cx| store.refresh_transactions(cx));
    crate::bills::ports::refresh(bills, ledger, cx);
}
