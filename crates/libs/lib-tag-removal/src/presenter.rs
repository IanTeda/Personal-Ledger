//! The Remove tag and Merge tags Dialogs' Presenters, moved from `lib-tags` (#1002). Each takes
//! `lib-viper`'s `DialogInput`s and its own events, builds a plain-data view model with its text
//! already localised from [`crate::msg`], and closes through the generic `DialogRouter` with
//! `lib-tags`' `TagsOutcome`. Each holds the shared removal port its confirm runs the Interactor
//! through, and is lent the reads once, when it opens.

mod merge_tags;
mod remove_tag;

pub use merge_tags::{MergeField, MergeTagsEvent, MergeTagsPresenter, MergeTagsViewModel};
pub use remove_tag::{RemoveTagEvent, RemoveTagPresenter, RemoveTagViewModel};

use crate::{Error, msg};

/// A refusal a Dialog shows.
fn refusal_text(error: Error) -> String {
    match error {
        Error::MergeIntoItself => msg::tags_refusal_merge_into_itself(),
        Error::NotFound => msg::tags_remove_refusal_gone(),
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::cell::RefCell;
    use std::rc::Rc;

    use crate::fake::InMemoryTagRemoval;
    use crate::fake::world::world;
    use crate::protocols::SharedTagRemoval;

    /// Composes this crate's layer into the process Locale, which every test that reads text
    /// needs: the first explicit `init` wins, so each test calls it before reading any.
    pub fn locale() {
        lib_locale::init_with_layers("en-US", &[crate::msg::LAYER]);
    }

    /// The world, shared, with the handle the test reads it back through.
    pub fn shared_world() -> (Rc<RefCell<InMemoryTagRemoval>>, SharedTagRemoval) {
        locale();
        world().shared()
    }
}
