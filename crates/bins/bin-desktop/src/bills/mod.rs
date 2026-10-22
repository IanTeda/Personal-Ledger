//! The Bills module's Desktop half (`docs/development/viper-architecture.md`): the Views for the
//! Bills page and its Dialogs, and `impl BillsRouter`. Every rule, every piece of text and the
//! keymap are `lib-bills`'; this folder turns gpui keys and clicks into Presenter events and draws
//! the view models.
//!
//! - `view/page.rs` wraps the page Presenter in an Entity, which the Shell hosts as the Bills
//!   destination; `view/schedule.rs`, `view/filters.rs` and `view/planner.rs` draw its tabs.
//! - `view/dialog.rs` hosts the Plan form's and Skip's Presenters in today's Dialog host, one View
//!   per Dialog in `view/dialog/`.
//! - `router.rs` is the Router and the module's entry point ([`bills_page`]).
//! - `ports.rs` holds the halfway shims: the reads copied from the Ledger, the store shared with
//!   `LedgerStore`, and the Ledger read copy refreshed after each write (#778).
//! - `pay_form.rs` and `apply.rs` are today's Pay Dialog, which the Router opens and the Shell still
//!   draws and confirms until `lib-bill-payments` takes it (#779).

pub(crate) mod apply;
pub(crate) mod pay_form;
pub mod ports;
mod router;
pub(crate) mod view;

pub use lib_bills::*;
pub use ports::{DesktopPorts, LedgerBackedPorts};
pub use router::bills_page;
pub use view::dialog::BillsDialog;
pub use view::page::BillsPage;
pub use view::snapshot::{BillRowSnapshot, BillsSnapshot};

use lib_bills::presenter::BillsPageEvent;

/// What `build_shell` hands the Bills module: its ports, shared with `LedgerStore`, and its keymap
/// under `[keybindings]`.
pub struct BillsSetup {
    pub ports: DesktopPorts,
    pub keymap: lib_viper::keymap::Keymap<BillsPageEvent>,
}

impl BillsSetup {
    pub fn new(ports: DesktopPorts, config: &lib_config::KeyBindingConfig) -> Self {
        Self {
            ports,
            keymap: keymap::bills_page_keymap(config),
        }
    }

    /// `storage`'s Bills ports under the default keys, for a test that builds `Shell` directly.
    #[cfg(test)]
    pub fn stub(storage: &crate::ledger_store::LedgerSetup) -> Self {
        Self::new(
            storage.bills.clone(),
            &lib_config::KeyBindingConfig::default(),
        )
    }
}

/// The calendar day the Bills page dates its rows by: `today` moving on with the local clock, so
/// a Desktop left running crosses into a new month, while a test pinned to a day stays on it.
pub fn clock(today: chrono::NaiveDate) -> presenter::Clock {
    let started = chrono::Local::now().date_naive();
    std::rc::Rc::new(move || today + (chrono::Local::now().date_naive() - started))
}
