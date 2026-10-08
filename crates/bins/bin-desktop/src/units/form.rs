//! The Add, Edit and Delete unit dialogs' live form state -- pure, `gpui`-free; `Shell` owns it
//! and `view::settings` renders it.

use super::{UnitKind, UnitRow};
use crate::form::field::TextField;

/// Which text field currently receives typed characters in the Add/Edit unit dialogs -- Type
/// has no equivalent variant since it's a click-select segmented control, not something you
/// type into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AddUnitField {
    #[default]
    Code,
    Name,
}

/// The Add/Edit unit dialogs' own live form state (issues #184/#185) -- pure, `gpui`-free,
/// mirroring `navigation/nav.rs`/`chrome/palette.rs`'s "state here, chrome renders it" split. `Shell` owns
/// `Option<Self>` wrapped in [`SettingsDialog`]; `None` means no dialog is open. Shared between
/// both dialogs rather than a separate `EditUnitForm` -- issue #185's own body: "same form as
/// Add unit" -- so [`Self::is_valid`]/[`Self::cycle_field`] only need writing once.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UnitForm {
    pub code: TextField,
    pub name: TextField,
    pub kind: UnitKind,
    pub focused_field: AddUnitField,
}

impl UnitForm {
    /// Pre-fills a form from an existing row (issue #185's own Edit unit dialog) -- `kind` maps
    /// back through [`UnitKind::from_label`] since the row stores a free-form string, not the
    /// enum itself.
    pub fn from_row(row: &UnitRow) -> Self {
        Self {
            code: TextField::new(row.code.as_str()),
            name: TextField::new(row.name.as_str()),
            kind: UnitKind::from_label(&row.kind),
            focused_field: AddUnitField::default(),
        }
    }

    /// The README's own "Dialog lifecycle" row: "fill Code / Name / Type (all required)" --
    /// Type always has a value (a segmented control can't be empty), so only Code/Name gate the
    /// Add/Save button's enabled state.
    pub fn is_valid(&self) -> bool {
        !self.code.is_blank() && !self.name.is_blank()
    }

    /// The field typing and `Backspace` edit.
    pub fn focused_mut(&mut self) -> &mut TextField {
        match self.focused_field {
            AddUnitField::Code => &mut self.code,
            AddUnitField::Name => &mut self.name,
        }
    }

    /// `Tab` cycles Code -> Name -> Code -- the dialog's own two-field focus ring, independent
    /// of `NavState::cycle_focus_forward`'s three shell-wide zones (`InputMode::Dialog` routes
    /// `Tab` here instead, before the shell-wide tier ever sees it).
    pub fn cycle_field(&mut self) {
        self.focused_field = match self.focused_field {
            AddUnitField::Code => AddUnitField::Name,
            AddUnitField::Name => AddUnitField::Code,
        };
    }
}

/// The Delete unit dialog's own live form state (issue #186) -- pure, `gpui`-free. Just the one
/// typed-back confirmation field: unlike [`UnitForm`], there's nothing to `Tab` between, so no
/// `focused_field` -- the confirmation input is implicitly the only thing you can type into
/// while this dialog is open. The code to type back is copied in when the dialog opens, so the
/// form validates without reading `Shell`'s Units (the dialog is modal: they can't change under
/// it).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeleteUnitForm {
    pub code: String,
    pub confirm_input: TextField,
}

impl DeleteUnitForm {
    pub fn new(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            confirm_input: TextField::default(),
        }
    }

    /// The README's own "disabled until the typed value matches the code exactly" -- a plain
    /// case-sensitive `==`, not a trim/lowercase-tolerant comparison, per the ticket's own body.
    pub fn is_valid(&self) -> bool {
        self.confirm_input.text() == self.code
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_form_from_row_prefills_every_field() {
        let row = UnitRow {
            code: "btc".to_string(),
            name: "Bitcoin".to_string(),
            kind: "crypto".to_string(),
            source: "CoinGecko".to_string(),
            is_base: false,
            is_default: false,
        };
        let form = UnitForm::from_row(&row);
        assert_eq!(form.code.text(), "btc");
        assert_eq!(form.name.text(), "Bitcoin");
        assert_eq!(form.kind, UnitKind::Custom);
        assert_eq!(form.focused_field, AddUnitField::Code);
    }

    #[test]
    fn add_unit_form_is_invalid_until_code_and_name_are_both_filled() {
        let mut form = UnitForm::default();
        assert!(!form.is_valid());
        form.focused_mut().push('a');
        assert!(!form.is_valid());
        form.cycle_field();
        form.focused_mut().push(' ');
        assert!(!form.is_valid());
        form.focused_mut().push('b');
        assert!(form.is_valid());
    }

    #[test]
    fn add_unit_form_cycle_field_toggles_between_code_and_name() {
        let mut form = UnitForm::default();
        assert_eq!(form.focused_field, AddUnitField::Code);
        form.cycle_field();
        assert_eq!(form.focused_field, AddUnitField::Name);
        form.cycle_field();
        assert_eq!(form.focused_field, AddUnitField::Code);
    }
}
