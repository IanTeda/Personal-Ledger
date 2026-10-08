//! The Add institution dialog's live form state -- pure, `gpui`-free; `Shell` owns it and
//! `view::settings` renders it.

use super::AccountType;
use crate::{form::field::TextField, units::UnitRow};

/// The Add institution dialog's own live form state (issue #187) -- pure, `gpui`-free.
/// Institution name is a real text field with no `focused_field`, mirroring
/// [`DeleteUnitForm`]'s own one-field shape: it's the only text field, so always implicitly
/// focused, nothing to `Tab` between. Account types are a real multi-select (more than one chip
/// may be checked at once, unlike [`UnitForm`]'s single-choice Type) -- `Vec` rather than a
/// fixed-size set since membership toggles are simpler as push/remove.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AddInstitutionForm {
    pub name: TextField,
    pub account_types: Vec<AccountType>,
    /// The selected default unit's own *code*, not an index into `Shell::settings_units` -- a
    /// code stays meaningful even if that `Vec`'s shape changes, though nothing in this map
    /// actually opens two dialogs at once for that to matter yet.
    pub default_unit_code: Option<String>,
}

impl AddInstitutionForm {
    /// Seeds Account types with the mockup's own `checked` default (savings) and Default unit
    /// with `units`' own first entry, if any -- the same "first option is the resting default"
    /// precedent `TracingLevel`/etc. already establish, just read from a runtime `Vec` instead
    /// of a compile-time enum's own `ALL`.
    pub fn new(units: &[UnitRow]) -> Self {
        Self {
            name: TextField::default(),
            account_types: vec![AccountType::Savings],
            default_unit_code: units.first().map(|unit| unit.code.clone()),
        }
    }

    /// Toggles `account_type`'s own membership -- present removes it, absent adds it.
    pub fn toggle_account_type(&mut self, account_type: AccountType) {
        match self
            .account_types
            .iter()
            .position(|&selected| selected == account_type)
        {
            Some(index) => {
                self.account_types.remove(index);
            }
            None => self.account_types.push(account_type),
        }
    }

    /// The README's own "Dialog lifecycle" row: "name + at least one account type + default
    /// unit" (all required).
    pub fn is_valid(&self) -> bool {
        !self.name.is_blank() && !self.account_types.is_empty() && self.default_unit_code.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::default_units;

    #[test]
    fn add_institution_form_new_seeds_savings_and_the_first_unit() {
        let units = default_units();
        let form = AddInstitutionForm::new(&units);
        assert_eq!(form.account_types, vec![AccountType::Savings]);
        assert_eq!(form.default_unit_code.as_deref(), Some("aud"));
    }

    #[test]
    fn add_institution_form_new_with_no_units_has_no_default_unit() {
        let form = AddInstitutionForm::new(&[]);
        assert_eq!(form.default_unit_code, None);
    }

    #[test]
    fn add_institution_form_toggle_account_type_adds_and_removes() {
        let mut form = AddInstitutionForm::new(&default_units());
        assert!(form.account_types.contains(&AccountType::Savings));
        form.toggle_account_type(AccountType::Savings);
        assert!(!form.account_types.contains(&AccountType::Savings));
        form.toggle_account_type(AccountType::Loan);
        assert!(form.account_types.contains(&AccountType::Loan));
    }
}
