//! The Categories module's Desktop half (`docs/development/viper-architecture.md`, #859): the Views
//! for the Categories Settings page and its Add and Edit Dialogs, and `impl CategoriesRouter`.
//! Every rule, every piece of text and the keymap are `lib-categories`'; this folder turns gpui
//! keys and clicks into Presenter events and draws the view models.
//!
//! - `view/page.rs` wraps the page Presenter in an Entity, which the Settings View hosts.
//! - `view/dialog.rs` holds the Add and Edit Dialog's Presenter in an Entity in the Dialog host.
//! - `router.rs` is the Router and the module's entry point ([`categories_page`]); it opens the
//!   Delete category Dialog through the Category Deletion module (`crate::category_deletion`,
//!   #1005).
//! - `ports.rs` holds the store as the Presenters share it, and the ports answered from the
//!   Ledger: the usage, the Monthly budget and the handover (#859).
//!
//! Where the Categories are kept, the SQLite adapter opened at start or memory, is
//! `lib-database`'s `CategoriesStore`.

pub mod ports;
mod router;
mod view;

pub use router::categories_page;
pub use view::page::CategoriesPage;

use lib_categories::presenter::CategoriesPageEvent;
use lib_core::{CategoryId, CategoryTypes};
use lib_database::SharedCategoriesStore;

use crate::category_deletion::DeletionStores;

/// The Categories page as the UI tests read it: names and types in the store's tree order, and the
/// selected row's name (ADR-0030).
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsCategoriesSnapshot {
    pub category_names: Vec<String>,
    /// Each Category's id, in the same order, for a test that clicks a row it didn't seed.
    pub category_ids: Vec<CategoryId>,
    /// Each Category's name and type, in the same order.
    pub category_types: Vec<(String, CategoryTypes)>,
    pub selected_category: Option<String>,
}

/// What `shell::router::build` hands the Categories module: the store it shares with the Ledger
/// and the Bills and Payees reads, Category delete's port and reads, and its keymap under
/// `[keybindings]`.
pub struct CategoriesSetup {
    pub store: SharedCategoriesStore,
    /// Category delete's port and reads, over the Categories store and the stores it re-points.
    pub deletion: DeletionStores,
    pub keymap: lib_viper::keymap::Keymap<CategoriesPageEvent>,
}

impl CategoriesSetup {
    pub fn new(
        store: SharedCategoriesStore,
        deletion: DeletionStores,
        config: &lib_config::KeyBindingConfig,
    ) -> Self {
        Self {
            store,
            deletion,
            keymap: lib_categories::keymap::categories_page_keymap(config),
        }
    }

    /// `storage`'s Categories store under the default keys, for a test that builds `Root`
    /// directly.
    #[cfg(test)]
    pub fn stub(storage: &crate::ledger_store::LedgerSetup) -> Self {
        Self::new(
            storage.categories.clone(),
            DeletionStores::stub(storage),
            &lib_config::KeyBindingConfig::default(),
        )
    }
}
