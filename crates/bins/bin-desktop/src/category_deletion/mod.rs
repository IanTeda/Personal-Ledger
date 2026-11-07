//! The Category Deletion module's Desktop half (`docs/development/viper-architecture.md`, #1005):
//! the Delete category Dialog's View and the entry point the Categories page's Router opens it
//! through. Every rule, every piece of text and the Presenter are `lib-category-deletion`'s; this
//! folder turns gpui keys and clicks into Presenter events and draws the view model.
//!
//! - `view/dialog.rs` holds the Presenter in an Entity in the Dialog host, drawn by
//!   `view/dialog/delete_category.rs`.
//! - `router.rs` is the module's entry point ([`router::open_delete`]).
//!
//! Delete writes through [`DeletionStores`]' port, `lib-database`'s adapter over the Categories,
//! Transactions, Bills, Payees and Budgets stores, so the Uncategorised it may
//! make, the re-pointed rows and the Category's delete join the one pending batch and land in the
//! one commit of the confirm's input. Nothing here reads the `Announcer`: the Categories page hears
//! of a confirm through its `landed` callback and brings what else reads those domains in line.

pub mod router;
pub(crate) mod view;

use lib_category_deletion::SharedCategoryDeletion;
use lib_database::StoredCategoryDeletionReads;

pub use view::dialog::CategoryDeletionDialog;

/// What the Delete category Dialog writes and reads through: the deletion port and the reads it
/// shows when it opens, which `lib-database`'s `LedgerPorts` builds over the shared stores.
#[derive(Clone)]
pub struct DeletionStores {
    pub deletion: SharedCategoryDeletion,
    pub reads: StoredCategoryDeletionReads,
}

impl DeletionStores {
    /// The port and reads `ports` built.
    pub fn of(ports: &lib_database::LedgerPorts) -> Self {
        Self {
            deletion: ports.category_deletion.clone(),
            reads: ports.category_deletion_reads.clone(),
        }
    }
}
