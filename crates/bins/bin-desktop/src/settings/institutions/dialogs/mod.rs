//! The Institutions page's Dialogs: Add institution. [`InstitutionsDialog`] is the value the Dialog
//! host holds while one is open (`OpenDialog::Institutions`). It draws itself (`Dialog::render`),
//! every click inside it reaching the form as a [`Change`] through the host's one edit callback,
//! and reads itself back for the UI tests.

pub mod add;

use std::rc::Rc;

use gpui::{AnyElement, App, Window};
use lib_units::UnitRow;

use super::form::{AddInstitutionForm, InstitutionProduct};
use crate::{
    chrome::dialog_host::{Dialog, DialogActions, DialogReadback, DialogScope},
    dialog,
    form::field::TextField,
};

/// The open Institutions Dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstitutionsDialog {
    Add(AddInstitutionForm),
}

/// Applies a [`Change`] to the open Institutions Dialog ([`InstitutionsDialog::apply_change`])
/// through the host's edit callback, then repaints.
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

impl InstitutionsDialogHandlers {
    /// A change edits the open Institutions Dialog through `actions`.
    fn new(actions: &DialogActions) -> Self {
        let on_change: OnChange = {
            let actions = actions.clone();
            Rc::new(move |change, _window, cx| {
                actions.edit(cx, |dialog: &mut InstitutionsDialog, _| {
                    dialog.apply_change(change);
                    None
                });
            })
        };
        Self {
            on_change,
            on_cancel: actions.cancel.clone(),
            on_confirm: actions.confirm.clone(),
        }
    }
}

impl InstitutionsDialog {
    /// Draws the open Dialog; `units` are the Default units it offers.
    fn draw(
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

    fn render(&self, scope: &DialogScope, actions: &DialogActions, cx: &App) -> Option<AnyElement> {
        Some(self.draw(
            scope.ledger.read(cx).ledger().units(),
            InstitutionsDialogHandlers::new(actions),
            cx,
        ))
    }

    /// Reads back the Name draft, the checked products (comma-joined `InstitutionProduct` variant
    /// names) and the chosen Default unit code.
    fn readback(&self, _scope: &DialogScope, _cx: &App) -> Option<DialogReadback> {
        let Self::Add(form) = self;
        Some(DialogReadback {
            kind: "AddInstitution",
            values: vec![
                ("name", form.name.text().to_string()),
                (
                    "types",
                    form.account_types
                        .iter()
                        .map(|product| format!("{product:?}"))
                        .collect::<Vec<_>>()
                        .join(","),
                ),
                ("unit", form.default_unit_code.clone().unwrap_or_default()),
            ],
        })
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
