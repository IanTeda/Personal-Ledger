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
//! A commit writes through `lib-database`'s `TransactionImportAdapter`, over the Payees and
//! Transactions stores, so its new Payees and Transactions join the one
//! pending batch and land in the commit of the input that continued.

pub(crate) mod router;
pub(crate) mod view;

use std::cell::RefCell;
use std::rc::Rc;

use lib_core::Clock;
use lib_database::{StoredImportReads, TransactionImportAdapter};
use lib_transaction_import::{ImportPorts, SeededStatement};

pub use view::page::ImportPage;

/// What the step reads and writes, which `lib-database`'s `LedgerPorts` built over the stores: the
/// import port (also its Payees read) and the Accounts and Categories reads. The seeded statement
/// is dated back from `clock`'s today when the step opens.
#[derive(Clone)]
pub struct ImportStores {
    pub import: Rc<RefCell<TransactionImportAdapter>>,
    pub reads: Rc<StoredImportReads>,
}

impl ImportStores {
    /// The port and reads `ports` built.
    pub fn of(ports: &lib_database::LedgerPorts) -> Self {
        Self {
            import: ports.transaction_import.clone(),
            reads: ports.import_reads.clone(),
        }
    }

    /// The ports the step opens on, its statement dated from `clock`'s today.
    pub(crate) fn ports(&self, clock: &Clock) -> ImportPorts {
        ImportPorts {
            import: self.import.clone(),
            payees: self.import.clone(),
            accounts: self.reads.clone(),
            categories: self.reads.clone(),
            statement: Rc::new(SeededStatement::stub(clock.today())),
        }
    }
}
