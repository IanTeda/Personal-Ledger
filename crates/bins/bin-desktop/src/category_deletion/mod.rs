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
//! Transactions, Bills, Payees and Budgets stores their modules share, so the Uncategorised it may
//! make, the re-pointed rows and the Category's delete join the one pending batch and land in the
//! one commit of the confirm's input. Nothing here reads the `Announcer`: the Categories page hears
//! of a confirm through its `landed` callback and brings what else reads those domains in line.

pub mod router;
pub(crate) mod view;

use std::cell::RefCell;
use std::rc::Rc;

use lib_category_deletion::SharedCategoryDeletion;
use lib_database::{
    CategoryDeletionAdapter, SharedBudgetsStore, SharedCategoriesStore, SharedPayeesStore,
    SharedTransactionsStore, StoredCategoryDeletionReads,
};

pub use view::dialog::CategoryDeletionDialog;

/// What the Delete category Dialog writes and reads through: the deletion port, and the reads it
/// shows when it opens, both over the shared stores.
#[derive(Clone)]
pub struct DeletionStores {
    pub deletion: SharedCategoryDeletion,
    pub reads: StoredCategoryDeletionReads,
}

impl DeletionStores {
    /// The port over `categories`, `transactions`, the Bills store `bills` writes, `payees` and
    /// `budgets`, and the reads over the Categories, Transactions and Budgets among them.
    pub fn new(
        categories: SharedCategoriesStore,
        transactions: SharedTransactionsStore,
        bills: &crate::bills::DesktopPorts,
        payees: SharedPayeesStore,
        budgets: SharedBudgetsStore,
    ) -> Self {
        Self {
            deletion: Rc::new(RefCell::new(CategoryDeletionAdapter::new(
                categories.clone(),
                transactions.clone(),
                bills.borrow().store(),
                payees,
                budgets.clone(),
            ))),
            reads: StoredCategoryDeletionReads {
                categories,
                transactions,
                budgets,
            },
        }
    }

    /// The port and reads over `storage`'s stores, for a test that builds `Root` directly.
    #[cfg(test)]
    pub fn sample(storage: &crate::shell::router::LedgerStores) -> Self {
        Self::new(
            storage.categories.clone(),
            storage.transactions.clone(),
            &storage.bills,
            storage.payees.clone(),
            storage.budgets.clone(),
        )
    }
}
