//! The Delete category Dialog's Presenter, copied from `lib-categories` (#1004). It takes
//! `lib-viper`'s `DialogInput`s and its own events, builds a plain-data view model with its text
//! already localised from [`crate::msg`], and closes through the generic `DialogRouter` with
//! `lib-categories`' `CategoriesOutcome`. It holds the shared deletion port its confirm runs the
//! Interactor through, and is lent the reads once, when it opens.

mod delete_category;

pub use delete_category::{DeleteCategoryEvent, DeleteCategoryPresenter, DeleteCategoryViewModel};

use crate::{Error, msg};

/// A refusal the Dialog shows.
fn refusal_text(error: Error) -> String {
    match error {
        Error::NotFound => msg::categories_refusal_gone(),
        Error::HasChildren => msg::categories_refusal_has_children(),
        Error::UncategorisedInUse => msg::categories_refusal_uncategorised_in_use(),
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::cell::RefCell;
    use std::rc::Rc;

    use crate::fake::InMemoryCategoryDeletion;
    use crate::fake::world::world;
    use crate::protocols::SharedCategoryDeletion;

    /// Composes this crate's layer into the process Locale, which every test that reads text
    /// needs: the first explicit `init` wins, so each test calls it before reading any.
    pub fn locale() {
        lib_locale::init_with_layers("en-US", &[crate::msg::LAYER]);
    }

    /// The world, shared, with the handle the test reads it back through.
    pub fn shared_world() -> (
        Rc<RefCell<InMemoryCategoryDeletion>>,
        SharedCategoryDeletion,
    ) {
        locale();
        world().shared()
    }
}
