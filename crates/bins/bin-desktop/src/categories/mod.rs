//! The Categories module's Desktop half (`docs/development/viper-architecture.md`, #859): the Views
//! for the Categories Settings page and its Add, Edit and Delete Dialogs, and `impl
//! CategoriesRouter`. Every rule, every piece of text and the keymap are `lib-categories`'; this
//! folder turns gpui keys and clicks into Presenter events and draws the view models.
//!
//! - `view/page.rs` wraps the page Presenter in an Entity, which the Settings View hosts.
//! - `view/dialog.rs` holds each Dialog's Presenter in an Entity in the Dialog host, one View per
//!   Dialog in `view/dialog/`.
//! - `router.rs` is the Router and the module's entry point ([`categories_page`]).
//! - `ports.rs` holds the store as the Presenters share it, the forwarding shim Category reassign
//!   still goes through, and the ports answered from the Ledger: the usage, the Monthly budget and
//!   the reassign port (#859).
//!
//! Where the Categories are kept, the SQLite adapter opened at start or memory, is
//! `lib-database`'s `CategoriesStore`.

pub mod ports;
mod router;
mod view;

pub use router::categories_page;
pub use view::page::CategoriesPage;

use lib_categories::presenter::CategoriesPageEvent;
use lib_core::CategoryTypes;
use lib_database::SharedCategoriesStore;

/// The Categories page as the UI tests read it: names and types in the store's tree order, and the
/// selected row's name (ADR-0030).
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsCategoriesSnapshot {
    pub category_names: Vec<String>,
    /// Each Category's name and type, in the same order.
    pub category_types: Vec<(String, CategoryTypes)>,
    pub selected_category: Option<String>,
}

/// What `shell::router::build` hands the Categories module: the store it shares with the Ledger
/// and the Bills and Payees reads, and its keymap under `[keybindings]`.
pub struct CategoriesSetup {
    pub store: SharedCategoriesStore,
    pub keymap: lib_viper::keymap::Keymap<CategoriesPageEvent>,
}

impl CategoriesSetup {
    pub fn new(store: SharedCategoriesStore, config: &lib_config::KeyBindingConfig) -> Self {
        Self {
            store,
            keymap: lib_categories::keymap::categories_page_keymap(config),
        }
    }

    /// `storage`'s Categories store under the default keys, for a test that builds `Root`
    /// directly.
    #[cfg(test)]
    pub fn stub(storage: &crate::ledger_store::LedgerSetup) -> Self {
        Self::new(
            storage.categories.clone(),
            &lib_config::KeyBindingConfig::default(),
        )
    }
}
