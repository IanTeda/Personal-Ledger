//! One Presenter per Document Types View: the Settings page and each of its Dialogs (Add and Edit
//! share the form). Each takes events through `handle` and builds a plain-data view model through
//! `view_model`, its text already localised from [`crate::msg`], so a View only draws. They share
//! one [`SharedRepository`](crate::protocols::SharedRepository) and ask for navigation through the
//! [`DocumentTypesRouter`](crate::protocols::DocumentTypesRouter) the Client passes in. Writes go
//! through [`crate::interactor`]'s Interactors.
//!
//! The Documents View isn't here yet (#967): this is the Document Types half of the module.

mod default_notice;
mod remove_type;
mod type_form;
mod types_page;

use crate::msg;
use crate::types::TracksDate;

pub use default_notice::{DefaultNoticeEvent, DefaultNoticePresenter, DefaultNoticeViewModel};
pub use remove_type::{RemoveTypeEvent, RemoveTypePresenter, RemoveTypeViewModel};
pub use type_form::{
    DocumentTypeField, NAME_MAX, NameError, REMIND_OPTIONS, TRACKS_OPTIONS, TypeFormEvent,
    TypeFormPresenter, TypeFormViewModel,
};
pub use types_page::{
    DocumentTypesPageEvent, DocumentTypesPagePresenter, DocumentTypesPageViewModel, TypeColumns,
    TypeLine,
};

/// The text of a Tracks date, in the page's cell and the form's options.
fn tracks_label(tracks: TracksDate) -> String {
    match tracks {
        TracksDate::Renews => msg::documents_types_tracks_renews(),
        TracksDate::Ends => msg::documents_types_tracks_ends(),
        TracksDate::Expires => msg::documents_types_tracks_expires(),
        TracksDate::Revalue => msg::documents_types_tracks_revalue(),
    }
}

/// A count as a Message's plural argument.
fn count(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::cell::RefCell;
    use std::rc::Rc;

    use crate::fake::InMemoryDocuments;
    use crate::protocols::SharedRepository;
    use crate::types::default_types;

    /// Composes this crate's layer into the process Locale, which every test that reads text
    /// needs: the first explicit `init` wins, so each test calls it before reading any.
    pub fn locale() {
        lib_locale::init_with_layers("en-US", &[crate::msg::LAYER]);
    }

    /// The nine default types, shared, with the handle the test reads them back through.
    pub fn stub() -> (Rc<RefCell<InMemoryDocuments>>, SharedRepository) {
        locale();
        let documents = Rc::new(RefCell::new(InMemoryDocuments {
            documents: Vec::new(),
            types: default_types(),
        }));
        let shared: SharedRepository = documents.clone();
        (documents, shared)
    }
}
