//! The Transaction Import module's Desktop half (`docs/development/viper-architecture.md`, #947):
//! the Import "match payees" step's View, shown in place of the Transactions list on `:import`, the
//! Noun staying Transactions. Every rule, every piece of text, the keymap and the Presenter are
//! `lib-transaction-import`'s; this folder turns gpui keys and clicks into Presenter events and
//! draws the view model.
//!
//! - `view/page.rs` holds the Presenter in an Entity while the step is open; `view/mod.rs` draws
//!   it.
//! - `router.rs` is `impl ImportRouter`: a close the page reads, and Notices onto the bin's queue.
//!
//! A commit writes through [`ports`]' `TransactionImportAdapter`, over the Payees and Transactions
//! stores the Payees and Transactions modules hold, so its new Payees and Transactions join the one
//! pending batch and land in the commit of the input that continued.

pub(crate) mod router;
pub(crate) mod view;

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{App, Entity};
use lib_core::Clock;
use lib_database::{SharedCategoriesStore, SharedTransactionsStore, TransactionImportAdapter};
use lib_transaction_import::{ImportPorts, SeededStatement};

use crate::ledger_store::LedgerStore;

pub use view::page::ImportPage;

/// What the step reads and writes, built once over the stores the other modules hold: the import
/// port and the Payees read over the Payees and Transactions stores, the Accounts and Categories
/// reads, and the seeded statement dated back from `clock`'s today when the step opens.
#[derive(Clone)]
pub struct ImportStores {
    pub payees: crate::payees::DesktopPorts,
    pub transactions: SharedTransactionsStore,
    pub accounts: crate::accounts::DesktopPorts,
    pub categories: SharedCategoriesStore,
}

impl ImportStores {
    /// The ports the step opens on, its statement dated from `clock`'s today.
    pub(crate) fn ports(&self, clock: &Clock) -> ImportPorts {
        let import = Rc::new(RefCell::new(TransactionImportAdapter::new(
            self.payees.clone(),
            self.transactions.clone(),
        )));
        let reads = Rc::new(lib_database::StoredImportReads {
            accounts: self.accounts.clone(),
            categories: self.categories.clone(),
        });
        ImportPorts {
            import: import.clone(),
            payees: import,
            accounts: reads.clone(),
            categories: reads,
            statement: Rc::new(SeededStatement::stub(clock.today())),
        }
    }
}

/// Brings the Ledger's Transactions read copy in line after a commit and announces the Payees, so
/// the Transactions list, the Dashboard and the Views reading the Payees see the Import at once.
/// The continuing input commits it.
pub(crate) fn refresh(ledger: &Entity<LedgerStore>, cx: &mut App) {
    ledger.update(cx, |store, cx| {
        store.refresh_transactions(cx);
        store.announce_payees(cx);
    });
}
