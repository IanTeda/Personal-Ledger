//! The Bill Payments module's Desktop half (`docs/development/viper-architecture.md`): the Pay
//! Dialog's View and the entry point another module's Router opens it through. Every rule, every
//! piece of text and the Presenter are `lib-bill-payments`'; this folder turns gpui keys and clicks
//! into Presenter events and draws the view model.
//!
//! - `view/dialog/pay.rs` holds the Pay Dialog's Presenter in an Entity in the Dialog host and draws it.
//! - `router.rs` is the module's entry point ([`router::open_pay`]), which the Bills Router calls.
//!
//! Pay it directly writes through `lib-database`'s `BillPaymentsAdapter` (`LedgerPorts`' `bill_payments`), over the Transactions
//! and Bills stores, so its Transaction and entry
//! join the one pending batch and land in one commit.

pub mod router;
pub(crate) mod view;

pub use view::dialog::pay::PayDialog;
