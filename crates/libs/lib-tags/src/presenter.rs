//! One Presenter per Tags View: the Settings page and each of its Dialogs (Add and Edit share the
//! form). Each takes events through `handle` and builds a plain-data view model through
//! `view_model`, its text already localised from [`crate::msg`], so a View only draws. They share
//! one set of ports ([`SharedPorts`](crate::protocols::SharedPorts)) and ask for navigation
//! through the [`TagsRouter`](crate::protocols::TagsRouter) the Client passes in. A Dialog's
//! Presenter instead takes `lib-viper`'s `DialogInput`s, reads the keys itself and closes through
//! the generic `DialogRouter` (#989). The Remove tag and Merge tags Dialogs are `lib-tag-removal`'s
//! (#1002), which closes them with this crate's `TagsOutcome`.

mod tag_form;
mod tags_page;

pub use tag_form::{
    ActiveField, ColourField, Swatch, TagField, TagFormEvent, TagFormPresenter, TagFormViewModel,
};
pub use tags_page::{TagLine, TagsPageEvent, TagsPagePresenter, TagsPageViewModel};

use crate::{TagError, msg};

/// A name's problem as the form words it, or the refusal it shows in its place: a blank name only
/// keeps Save from confirming, so it has no text of its own here.
fn name_error_text(error: &TagError) -> String {
    match error {
        TagError::DuplicateName(name) => msg::tags_error_name_taken(name),
        TagError::NoLetterOrDigit => msg::tags_error_no_letter_or_digit(),
        TagError::NotFound => msg::tags_refusal_gone(),
        TagError::NameRequired => {
            debug_assert!(
                false,
                "a blank name has no text: it only keeps Save from confirming"
            );
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

    /// The sample seed, shared, with the handle the test reads it back through.
    pub fn sample() -> (Rc<RefCell<InMemoryPorts>>, SharedPorts) {
        locale();
        InMemoryPorts::sample().shared()
    }
}
