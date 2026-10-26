//! One Presenter per Tags View: the Settings page and each of its Dialogs (Add and Edit share the
//! form). Each takes events through `handle` and builds a plain-data view model through
//! `view_model`, its text already localised from [`crate::msg`], so a View only draws. They share
//! one set of ports ([`SharedPorts`](crate::protocols::SharedPorts)) and ask for navigation
//! through the [`TagsRouter`](crate::protocols::TagsRouter) the Client passes in. A Dialog's
//! confirm is lent the reassign port for Remove and Merge, which writes through the Client's
//! Ledger.

mod merge_tags;
mod remove_tag;
mod tag_form;
mod tags_page;

pub use merge_tags::{MergeField, MergeTagsEvent, MergeTagsPresenter, MergeTagsViewModel};
pub use remove_tag::{RemoveTagEvent, RemoveTagPresenter, RemoveTagViewModel};
pub use tag_form::{
    ActiveField, ColourField, Swatch, TagField, TagFormEvent, TagFormPresenter, TagFormViewModel,
};
pub use tags_page::{Hint, TagLine, TagsPageEvent, TagsPagePresenter, TagsPageViewModel};

use crate::{TagError, msg};

/// A count as a Message's plural argument.
fn count(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// A name's problem as the form words it. A blank name only keeps Save from confirming, and the
/// rest come from Remove and Merge, so none has text of its own here.
fn name_error_text(error: &TagError) -> String {
    match error {
        TagError::DuplicateName(name) => msg::tags_error_name_taken(name),
        TagError::NoLetterOrDigit => msg::tags_error_no_letter_or_digit(),
        other => refusal_text(other),
    }
}

/// A refusal a Dialog shows in place of a name's problem.
fn refusal_text(error: &TagError) -> String {
    match error {
        TagError::MergeIntoItself => msg::tags_refusal_merge_into_itself(),
        TagError::NotFound => msg::tags_refusal_gone(),
        other => {
            debug_assert!(false, "no Tags Dialog shows {other:?} as a refusal");
            msg::tags_refusal_gone()
        }
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::cell::RefCell;
    use std::rc::Rc;

    use crate::fake::InMemoryPorts;
    use crate::protocols::SharedPorts;

    /// Composes this crate's layer into the process Locale, which every test that reads text
    /// needs: the first explicit `init` wins, so each test calls it before reading any.
    pub fn locale() {
        lib_locale::init_with_layers("en-US", &[crate::msg::LAYER]);
    }

    /// The stub seed, shared, with the handle the test reads it back through.
    pub fn stub() -> (Rc<RefCell<InMemoryPorts>>, SharedPorts) {
        locale();
        InMemoryPorts::stub().shared()
    }
}
