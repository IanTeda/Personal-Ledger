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
//! The ports are `lib-categories`' own, which `LedgerPorts` hands the page and its Dialogs: the
//! repository, the save ports (the usage, the Monthly budget and the handover) and the months a
//! Monthly budget has a record in, which the page's snapshot reads (#859, #1083, #1207).

mod router;
mod view;

pub use router::categories_page;

use lib_categories::presenter::CategoriesPagePresenter;
use lib_core::{CategoryId, CategoryTypes};

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

/// What the Categories page's Router is built from for each call (its `PageView::Setup`): the
/// shared Categories, the save ports each form holds from open, the Budget months the page's
/// snapshot reads, Category delete's port and reads, and the Dialog keys every Categories Dialog's
/// hints read.
pub struct CategoriesSetup {
    pub categories: lib_categories::SharedCategories,
    /// A value of its own over the stores, so a form writing through it never meets a borrow of
    /// `categories` the page holds.
    pub save: lib_categories::SharedSavePorts,
    /// The months the selected Category has a Budget Amount record in, for the page's snapshot.
    pub budget_months: lib_categories::SharedBudgetMonths,
    /// Category delete's port and reads, over the Categories store and the stores it re-points.
    pub deletion: lib_category_deletion::CategoryDeletionPorts,
    pub dialog_keys: crate::dialog_keys::LiveDialogKeys,
}

impl CategoriesSetup {
    /// The page's Presenter, which owns the Categories keymap under `[keybindings]`, and its
    /// Setup.
    pub fn new(
        categories: lib_categories::SharedCategories,
        save: lib_categories::SharedSavePorts,
        budget_months: lib_categories::SharedBudgetMonths,
        deletion: lib_category_deletion::CategoryDeletionPorts,
        dialog_keys: crate::dialog_keys::LiveDialogKeys,
        config: &lib_core::configuration::KeyBindingConfig,
    ) -> (CategoriesPagePresenter, Self) {
        let presenter = CategoriesPagePresenter::new(categories.clone(), config);
        let setup = Self {
            categories,
            save,
            budget_months,
            deletion,
            dialog_keys,
        };
        (presenter, setup)
    }

    /// `ports`' Categories store under the default keys, for a test that builds `Root`
    /// directly.
    #[cfg(test)]
    pub fn sample(ports: &lib_database::LedgerPorts) -> (CategoriesPagePresenter, Self) {
        let config = lib_core::configuration::KeyBindingConfig::default();
        Self::new(
            ports.categories(),
            ports.category_save(),
            ports.category_budget_months(),
            ports.category_deletion(),
            crate::dialog_keys::LiveDialogKeys::new(lib_viper::dialog::DialogKeys::of(
                &lib_shell::dialog_keymap(&config),
            )),
            &config,
        )
    }
}
