//! One Presenter per Inventory View: the Settings page and each of its Dialogs (Add and Edit share
//! a form for a Location, and another for an Area). Each takes events through `handle` and builds
//! a plain-data view model through `view_model`, its text already localised from [`crate::msg`], so
//! a View only draws. They share the repository ([`SharedInventory`]) and ask for navigation
//! through the [`InventoryRouter`] the Client passes in. A Dialog opens with the reads of other
//! domains it quotes lent to it.
//!
//! [`SharedInventory`]: crate::protocols::SharedInventory
//! [`InventoryRouter`]: crate::protocols::InventoryRouter

mod area_blocked;
mod area_form;
mod inventory_page;
mod location_form;
mod remove_area;
mod remove_location;

pub use area_blocked::{AreaBlockedEvent, AreaBlockedPresenter, AreaBlockedViewModel};
pub use area_form::{AreaFormEvent, AreaFormPresenter, AreaFormViewModel};
pub use inventory_page::{
    InventoryLine, InventoryPageEvent, InventoryPagePresenter, InventoryPageViewModel, MenuAction,
    MenuEntry, MenuView,
};
pub use location_form::{
    LocationField, LocationFormEvent, LocationFormPresenter, LocationFormViewModel, UnitField,
};
pub use remove_area::{RemoveAreaEvent, RemoveAreaPresenter, RemoveAreaViewModel};
pub use remove_location::{RemoveLocationEvent, RemoveLocationPresenter, RemoveLocationViewModel};

use crate::{Error, NameError, msg};

/// A Location name's problem as the form words it.
fn location_name_error_text(error: NameError) -> String {
    match error {
        NameError::Empty => msg::inventory_error_name_empty(),
        NameError::Taken => msg::inventory_error_name_taken(),
    }
}

/// An Area name's problem as the form words it.
fn area_name_error_text(error: NameError) -> String {
    match error {
        NameError::Empty => msg::inventory_area_error_name_empty(),
        NameError::Taken => msg::inventory_area_error_name_taken(),
    }
}

/// A refusal a Location Dialog shows in place of a name's problem. The forms check names live, so
/// what is left is a row that went, or changed, while the Dialog was open.
fn location_refusal_text(error: Error) -> String {
    match error {
        Error::Name(error) => location_name_error_text(error),
        Error::UnknownLocation | Error::Cover(_) => msg::inventory_refusal_gone(),
        error @ (Error::UnknownArea
        | Error::UnknownItem
        | Error::ItemNameEmpty
        | Error::ReplacementNegative
        | Error::NeedsDestination
        | Error::BadDestination
        | Error::LastAreaHoldsItems) => {
            debug_assert!(false, "a Location write refused with {error:?}");
            tracing::error!(error = ?error, "a Location write was refused with an error it never returns");
            msg::inventory_refusal_gone()
        }
    }
}

/// A refusal an Area Dialog shows.
fn area_refusal_text(error: Error) -> String {
    match error {
        Error::Name(error) => area_name_error_text(error),
        Error::UnknownLocation
        | Error::UnknownArea
        | Error::NeedsDestination
        | Error::BadDestination
        | Error::LastAreaHoldsItems => msg::inventory_refusal_gone(),
        error @ (Error::UnknownItem
        | Error::Cover(_)
        | Error::ItemNameEmpty
        | Error::ReplacementNegative) => {
            debug_assert!(false, "an Area write refused with {error:?}");
            tracing::error!(error = ?error, "an Area write was refused with an error it never returns");
            msg::inventory_refusal_gone()
        }
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::cell::RefCell;
    use std::rc::Rc;

    use lib_viper::dialog::{DialogKeys, dialog_keymap};
    use lib_viper::keymap::KeyBindingConfig;

    use crate::fake::InMemoryInventory;
    use crate::protocols::SharedInventory;

    /// Composes this crate's layer into the process Locale, which every test that reads text
    /// needs: the first explicit `init` wins, so each test calls it before reading any.
    pub fn locale() {
        lib_locale::init_with_layers("en-US", &[crate::msg::LAYER]);
    }

    /// The sample seed, shared, with the handle the test reads it back through.
    pub fn sample() -> (Rc<RefCell<InMemoryInventory>>, SharedInventory) {
        locale();
        InMemoryInventory::sample(
            chrono::NaiveDate::from_ymd_opt(2026, 10, 2).expect("a real date"),
        )
        .shared()
    }

    /// The Dialog keys a Dialog's legend reads, bound as `lib-shell`'s defaults bind them.
    pub fn dialog_keys() -> DialogKeys {
        DialogKeys::of(&dialog_keymap(&KeyBindingConfig::default()))
    }
}
