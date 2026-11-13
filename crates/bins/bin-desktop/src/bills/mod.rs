//! The Bills module's Desktop half (`docs/development/viper-architecture.md`): the Views for the
//! Bills page and its Dialogs, and `impl BillsRouter`. Every rule, every piece of text and the
//! keymap are `lib-bills`'; this folder turns gpui keys and clicks into Presenter events and draws
//! the view models.
//!
//! - `view/page.rs` is the page Presenter's `PageView`, which the generic `ModulePage` hosts as the
//!   Bills destination; `view/schedule.rs`, `view/filters.rs` and `view/planner.rs` draw its tabs.
//! - `view/dialog/` holds the Plan form's and Skip's `DialogView`s, each hosted in a
//!   `ModuleDialog`.
//! - `router.rs` is the Router and the module's entry point ([`bills_page`]). It opens the Pay
//!   Dialog through the Bill Payments module's entry point (`crate::bill_payments`).
//!
//! The ports are `lib-database`'s `StoredBillsPorts`, which `LedgerPorts` hands the page and its
//! Dialogs (#1084); what stays here is when the Bill Schedule is generated ([`generate`],
//! [`new_month`]).

mod router;
pub(crate) mod view;

pub use lib_bills::*;
pub use router::bills_page;
pub use view::page::BillsPage;
pub use view::snapshot::{BillRowSnapshot, BillsSnapshot};

use lib_bills::interactor::PopulateSchedule;

/// Brings the stored Bill Schedule up to the horizon as of `today` (ADR-0019's "on Client start"),
/// writing the entries generation adds. The caller commits them.
pub fn generate(store: &mut dyn BillsRepository, today: chrono::NaiveDate) {
    PopulateSchedule { today }.run(store);
}

/// Brings the Bill Schedule up to the horizon for the month a running Desktop has crossed into,
/// wherever it is (ADR-0019). The day poll's input commits the new entries as one write, which
/// the input door announces, so the Dashboard and Budgets redraw, and a failed save rolls back like
/// any Bills write.
pub fn new_month(ports: &SharedPorts, today: chrono::NaiveDate) {
    PopulateSchedule { today }.run(ports.borrow_mut().split_mut().bills);
}

/// What `shell::router::build` hands the Bills module: its ports, shared with the Open Ledger, the payments
/// port the Pay Dialog it opens writes through and the Dialog keys its Dialogs' hints show.
pub struct BillsSetup {
    pub ports: SharedPorts,
    pub payments: lib_bill_payments::SharedPayments,
    pub dialog_keys: crate::dialog_keys::LiveDialogKeys,
}

impl BillsSetup {
    pub fn new(
        ports: SharedPorts,
        payments: lib_bill_payments::SharedPayments,
        dialog_keys: crate::dialog_keys::LiveDialogKeys,
    ) -> Self {
        Self {
            ports,
            payments,
            dialog_keys,
        }
    }

    /// `storage`'s Bills ports and payments, for a test that builds `Root` directly.
    #[cfg(test)]
    pub fn sample(storage: &crate::shell::router::LedgerStores) -> Self {
        Self::new(
            storage.ports.bills.clone(),
            storage.ports.bill_payments.clone(),
            crate::dialog_keys::LiveDialogKeys::default(),
        )
    }
}
