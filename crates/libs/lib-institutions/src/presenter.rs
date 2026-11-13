//! One Presenter per Institutions View: the Settings page and its Add, Edit and Delete Dialogs.
//! Each keeps `lib-viper`'s Presenter contract (ADR-0056): it takes events through `handle` and builds a plain-data view model through `view_model`, its
//! text already localised from [`crate::msg`], so a View only draws. They share one set of ports
//! ([`SharedPorts`](crate::protocols::SharedPorts)) and ask for navigation through the
//! [`InstitutionsRouter`](crate::protocols::InstitutionsRouter) the Client passes in.

mod delete_institution;
mod institution_form;
mod institutions_page;

pub use delete_institution::{
    DeleteInstitutionEvent, DeleteInstitutionPresenter, DeleteInstitutionViewModel,
};
pub use institution_form::{
    InstitutionField, InstitutionFormEvent, InstitutionFormPresenter, InstitutionFormViewModel,
    InstitutionProduct,
};
pub use institutions_page::{
    InstitutionColumns, InstitutionLine, InstitutionsPageEvent, InstitutionsPagePresenter,
    InstitutionsPageViewModel, MenuAction, MenuEntry, RowMenu,
};

#[cfg(test)]
pub(crate) mod test_support {
    use std::cell::RefCell;
    use std::rc::Rc;

    use lib_viper::dialog::{DialogKeys, dialog_keymap};
    use lib_viper::keymap::KeyBindingConfig;

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

    /// The Dialog keys a Dialog's legend reads, bound as `lib-shell`'s defaults bind them.
    pub fn dialog_keys() -> DialogKeys {
        DialogKeys::of(&dialog_keymap(&KeyBindingConfig::default()))
    }
}
