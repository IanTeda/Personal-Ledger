//! The Categories module's Desktop half (`docs/development/viper-architecture.md`, #859): the Views
//! for the Categories Settings page and its Add and Edit Dialogs, and `impl CategoriesRouter`.
//! Every rule, every piece of text and the keymap are `lib-categories`'; this folder turns gpui
//! keys and clicks into Presenter events and draws the view models.
//!
//! - `view/page.rs` is the page Presenter's `PageView`: the generic `ModulePage` hosts it
//!   ([`CategoriesPage`]), which the Settings View hosts as any other page (ADR-0056).
//! - `view/dialog.rs` and `view/dialog/` are the Add and Edit form Presenter's `DialogView`, hosted
//!   in a `ModuleDialog`.
//! - `router.rs` is the Router and the module's entry point ([`categories_page`]); it opens the
//!   Delete category Dialog through the Category Deletion module (`crate::category_deletion`,
//!   #1005).
//!
//! The ports are `lib-database`'s `StoredCategoriesPorts`, which `LedgerPorts` hands the page and
//! its Dialogs: the repository, the usage, the Monthly budget and the handover (#859, #1083).

mod router;
mod view;

pub use router::categories_page;

use lib_categories::presenter::CategoriesPagePresenter;
use lib_core::{CategoryId, CategoryTypes};

use crate::category_deletion::DeletionStores;
use crate::shell::view::module_page::ModulePage;

/// The Categories Settings page: the page Presenter in the generic page Entity.
pub type CategoriesPage = ModulePage<CategoriesPagePresenter>;

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
    /// The months, as `(year, month)`, in which the selected Category has a Budget Amount record
    /// in the Personal spending Budget, so a test sees which month a save started in.
    pub selected_budget_months: Vec<(i32, u32)>,
}

/// What the Open Ledger hands the Categories module: the page's Presenter, which owns the
/// Categories keymap under `[keybindings]`, `lib-database`'s Categories ports as the save ports each
/// form holds from open, the Budgets store the page's snapshot reads, Category delete's port and
/// reads, and the Dialog keys every Categories Dialog's hints read.
pub struct CategoriesSetup {
    pub presenter: CategoriesPagePresenter,
    pub categories: lib_categories::SharedCategories,
    pub save: lib_database::StoredCategoriesPorts,
    /// The Budgets store, which the page's snapshot reads the selected Category's records from.
    pub budgets: lib_database::SharedBudgetsStore,
    /// Category delete's port and reads, over the Categories store and the stores it re-points.
    pub deletion: DeletionStores,
    pub dialog_keys: crate::dialog_keys::LiveDialogKeys,
}

impl CategoriesSetup {
    pub fn new(
        categories: lib_categories::SharedCategories,
        save: lib_database::StoredCategoriesPorts,
        budgets: lib_database::SharedBudgetsStore,
        deletion: DeletionStores,
        dialog_keys: crate::dialog_keys::LiveDialogKeys,
        config: &lib_config::KeyBindingConfig,
    ) -> Self {
        Self {
            presenter: CategoriesPagePresenter::new(categories.clone(), config),
            categories,
            save,
            budgets,
            deletion,
            dialog_keys,
        }
    }

    /// `storage`'s Categories store under the default keys, for a test that builds `Root`
    /// directly.
    #[cfg(test)]
    pub fn sample(storage: &crate::shell::router::LedgerStores) -> Self {
        let config = lib_config::KeyBindingConfig::default();
        Self::new(
            storage.ports.categories.clone(),
            storage.ports.category_save.clone(),
            storage.ports.stores.budgets.clone(),
            DeletionStores::of(&storage.ports),
            crate::dialog_keys::LiveDialogKeys::new(lib_viper::dialog::DialogKeys::of(
                &lib_shell::dialog_keymap(&config),
            )),
            &config,
        )
    }
}

/// What the Categories page's Router is built from for each call (its `PageView::Setup`): the
/// ports and Dialog keys its Dialogs open on, and the Budgets store its snapshot reads.
pub struct CategoriesRoutes {
    pub categories: lib_categories::SharedCategories,
    pub save: lib_database::StoredCategoriesPorts,
    pub budgets: lib_database::SharedBudgetsStore,
    pub deletion: DeletionStores,
    pub dialog_keys: crate::dialog_keys::LiveDialogKeys,
}
