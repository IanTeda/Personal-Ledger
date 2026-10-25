//! One Presenter per Payees View: the Settings page and each of its Dialogs (Add and Edit share the
//! form; Delete, Deactivate and Reactivate share the Delete Dialog). Each takes events through
//! `handle` and builds a plain-data view model through `view_model`, its text already localised
//! from [`crate::msg`], so a View only draws. They share one set of ports
//! ([`SharedPorts`](crate::protocols::SharedPorts)) and ask for navigation through the
//! [`PayeesRouter`](crate::protocols::PayeesRouter) the Client passes in.

mod delete_payee;
mod payee_form;
mod payees_page;

pub use delete_payee::{
    DeleteAction, DeletePayeeEvent, DeletePayeePresenter, DeletePayeeViewModel,
};
pub use payee_form::{Callout, PayeeField, PayeeFormEvent, PayeeFormPresenter, PayeeFormViewModel};
pub use payees_page::{
    Hint, PayeeColumns, PayeeLine, PayeesPageEvent, PayeesPagePresenter, PayeesPageViewModel,
};

use crate::{PayeeError, msg};

/// A count as a Message's plural argument.
fn count(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// A refusal as the Dialogs show it. A blank name only keeps Save from confirming, and in-use
/// comes back from Delete as its own refusal, so neither has text of its own.
fn refusal_text(error: &PayeeError) -> String {
    match error {
        PayeeError::DuplicateName(name) => msg::payees_error_name_taken(name),
        PayeeError::AliasTaken { alias, owner } => msg::payees_error_alias_taken(alias, owner),
        PayeeError::NotFound => msg::payees_refusal_gone(),
        refusal @ PayeeError::NameRequired => {
            debug_assert!(false, "no Payees Dialog shows {refusal:?}");
            msg::payees_refusal_gone()
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
