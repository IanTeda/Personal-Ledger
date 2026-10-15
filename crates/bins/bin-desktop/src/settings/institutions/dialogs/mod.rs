//! The Institutions page's Dialogs: Add institution. [`InstitutionsDialog`] is the value `Shell`'s
//! Dialog host holds while one is open (`OpenDialog::Institutions`); [`InstitutionsDialog::render`]
//! draws it, and every click inside it reaches the form as a [`Change`].

pub mod add;

use std::rc::Rc;

use gpui::{AnyElement, App, Window};
use lib_units::UnitRow;

use super::form::{AddInstitutionForm, InstitutionProduct};
use crate::{chrome::dialog_host::Dialog, dialog, form::field::TextField};

/// The open Institutions Dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstitutionsDialog {
    Add(AddInstitutionForm),
}

/// Applies a [`Change`] to the open Institutions Dialog where the Dialog host holds it
/// ([`InstitutionsDialog::apply_change`]), then repaints.
pub type OnChange = Rc<dyn Fn(Change, &mut Window, &mut App)>;

/// What a click inside an Institutions Dialog changes on its form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// Toggles an Account type chip.
    AccountType(InstitutionProduct),
    /// Picks the Default unit by its code: the Units table is the Ledger's, so an index would
    /// need re-deriving against it anyway.
    Unit(String),
}

/// The handlers an Institutions Dialog binds: changes to the open form, cancel and confirm.
#[derive(Clone)]
pub struct InstitutionsDialogHandlers {
    pub on_change: OnChange,
    pub on_cancel: dialog::OnClick,
    pub on_confirm: dialog::OnClick,
}

impl InstitutionsDialog {
    /// Draws the open Dialog; `units` are the Default units it offers.
    pub fn render(
        &self,
        units: &[UnitRow],
        handlers: InstitutionsDialogHandlers,
        cx: &App,
    ) -> AnyElement {
        match self {
            Self::Add(form) => add::render(form, units, handlers, cx),
        }
    }

    /// Applies a click's change to the form.
    pub fn apply_change(&mut self, change: Change) {
        let Self::Add(form) = self;
        match change {
            Change::AccountType(account_type) => form.toggle_account_type(account_type),
            Change::Unit(code) => form.default_unit_code = Some(code),
        }
    }
}

impl Dialog for InstitutionsDialog {
    fn focused_text(&mut self) -> Option<&mut TextField> {
        let Self::Add(form) = self;
        Some(&mut form.name)
    }

    fn is_valid(&self) -> bool {
        let Self::Add(form) = self;
        form.is_valid()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chrome::dialog_host::{OpenDialog, type_text};
    use lib_units::default_units;

    #[test]
    fn add_institution_needs_a_name_an_account_type_and_a_unit() {
        let mut dialog = OpenDialog::Institutions(InstitutionsDialog::Add(
            AddInstitutionForm::new(&default_units()),
        ));
        assert!(!dialog.is_valid());
        type_text(&mut dialog, "ANZ");
        assert!(dialog.is_valid());
        if let OpenDialog::Institutions(InstitutionsDialog::Add(form)) = &mut dialog {
            form.toggle_account_type(InstitutionProduct::Savings);
        }
        assert!(!dialog.is_valid());
    }

    #[test]
    fn a_click_change_toggles_a_chip_or_picks_the_unit() {
        let mut dialog = InstitutionsDialog::Add(AddInstitutionForm::new(&default_units()));
        dialog.apply_change(Change::AccountType(InstitutionProduct::Loan));
        dialog.apply_change(Change::Unit("btc".into()));
        let InstitutionsDialog::Add(form) = &dialog;
        assert!(form.account_types.contains(&InstitutionProduct::Loan));
        assert_eq!(form.default_unit_code.as_deref(), Some("btc"));
    }
}
