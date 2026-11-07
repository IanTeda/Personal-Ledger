//! One Presenter per Accounts View: the Settings page and the Add and Edit Dialogs (one form).
//! Each takes events through `handle` and builds a plain-data view model through `view_model` (`lib-viper`'s Presenter contract, ADR-0056), its
//! text already localised from [`crate::msg`], so a View only draws. They share one set of ports
//! ([`SharedPorts`](crate::protocols::SharedPorts)) and ask for navigation through the
//! [`AccountsRouter`](crate::protocols::AccountsRouter) the Client passes in. Delete is
//! `lib-account-deletion`'s Dialog, which the page's Router opens.

mod account_form;
mod accounts_page;

pub use account_form::{
    AccountField, AccountFormEvent, AccountFormPresenter, AccountFormViewModel, Usage,
};
pub use accounts_page::{
    AccountColumns, AccountGroupView, AccountLine, AccountsCommand, AccountsPageEvent,
    AccountsPagePresenter, AccountsPageViewModel,
};

use lib_core::InstitutionId;

use crate::protocols::AccountInstitution;

/// What to show for an Account's `institution`: the name of the one of `institutions` it names,
/// unchanged (Ledger data, not a Message). An id `institutions` doesn't hold shows nothing.
fn institution_label(institution: InstitutionId, institutions: &[AccountInstitution]) -> String {
    institutions
        .iter()
        .find(|row| row.id == institution)
        .map_or_else(String::new, |row| row.name.clone())
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::cell::RefCell;
    use std::rc::Rc;

    use lib_viper::dialog::DialogKeys;
    use lib_viper::keymap::{BACK, Binding, KeyBindingConfig, Keymap, NEXT_FIELD, SELECT};

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
        const BINDINGS: &[Binding<()>] = &[
            Binding::hidden(BACK, "esc", ()),
            Binding::hidden(SELECT, "enter", ()),
            Binding::hidden(NEXT_FIELD, "tab", ()),
        ];
        DialogKeys::of(&Keymap::new(BINDINGS, &KeyBindingConfig::default()))
    }
}
