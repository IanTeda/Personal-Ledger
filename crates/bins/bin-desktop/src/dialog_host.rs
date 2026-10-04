//! The Dialog host: the one slot a Dialog lives in while it is open, and the keyboard handling
//! every Dialog shares. `gpui`-free, so a Dialog's keys are unit-tested without a window.
//!
//! `Shell` holds `Option<OpenDialog>` and opens and closes it only through `Shell::open_dialog`
//! and `Shell::close_dialog`, which also enter and leave `InputMode::Dialog`, so the slot and
//! the mode can't disagree. A key reaches the open Dialog through [`handle_key`]: the Dialog's
//! own [`Dialog::handle_own_key`] sees it first (a list Dialog's `j`/`k`, a search box), then
//! the shared handling types into the focused [`TextField`], cycles fields on `Tab` and confirms
//! on `Enter` only while the form is valid. A Dialog never touches ledger data: it answers
//! [`DialogOutcome::Confirm`] and `Shell` applies the form (`Shell::confirm_open_dialog`), the
//! same path the confirm button's click takes.
//!
//! `Esc` is not a key here: the router turns it into `ClosePopupsAndExitMode`, and `Shell` asks
//! [`Dialog::close_open_select`] first, closing the Dialog only when no select was open.
//!
//! Drawing stays in `crate::dialog` (the chrome) and each feature's view; `Shell::render` matches
//! on the [`OpenDialog`] variant to draw it.
//!
//! Dialogs move into this slot one feature at a time; until they all have, the rest still live in
//! their own `Shell::*_dialog` fields.

use crate::{field::TextField, settings::SettingsDialog};

/// A keystroke as a Dialog sees it, already stripped of modifiers by `Shell`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogKey {
    Backspace,
    Tab,
    Enter,
    /// A printable character, typed without Ctrl/Alt/Cmd/Fn.
    Char(char),
    /// Anything else (arrows, function keys, a modified chord).
    Other,
}

/// What `Shell` does after a Dialog has seen a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogOutcome {
    /// Not the Dialog's key; `Shell` lets it fall through.
    Ignored,
    Handled,
    /// The form is valid and the user confirmed: `Shell` applies it and closes the Dialog.
    Confirm,
}

/// One feature's Dialogs. Every method has the default a plain confirm (no fields) wants.
pub trait Dialog {
    /// Keys this Dialog takes before the shared handling, `None` to leave the key to it.
    fn handle_own_key(&mut self, _key: DialogKey) -> Option<DialogOutcome> {
        None
    }

    /// The text field that typing and `Backspace` edit.
    fn focused_text(&mut self) -> Option<&mut TextField> {
        None
    }

    /// `Tab`: the next field in this Dialog's own focus ring. Never reaches the shell-wide zones.
    fn cycle_field(&mut self) {}

    /// Whether `Enter` (and the confirm button) may confirm.
    fn is_valid(&self) -> bool {
        true
    }

    /// The first `Esc` closes an open select only. `true` when one was open.
    fn close_open_select(&mut self) -> bool {
        false
    }
}

/// The open Dialog, one variant per feature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenDialog {
    Settings(SettingsDialog),
}

impl OpenDialog {
    fn inner(&self) -> &dyn Dialog {
        match self {
            Self::Settings(dialog) => dialog,
        }
    }

    fn inner_mut(&mut self) -> &mut dyn Dialog {
        match self {
            Self::Settings(dialog) => dialog,
        }
    }
}

impl Dialog for OpenDialog {
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        self.inner_mut().handle_own_key(key)
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        self.inner_mut().focused_text()
    }

    fn cycle_field(&mut self) {
        self.inner_mut().cycle_field();
    }

    fn is_valid(&self) -> bool {
        self.inner().is_valid()
    }

    fn close_open_select(&mut self) -> bool {
        self.inner_mut().close_open_select()
    }
}

/// Routes `key` to `dialog`: its own keys first, then typing, `Tab` and `Enter`.
pub fn handle_key(dialog: &mut impl Dialog, key: DialogKey) -> DialogOutcome {
    if let Some(outcome) = dialog.handle_own_key(key) {
        return outcome;
    }
    match key {
        DialogKey::Backspace => match dialog.focused_text() {
            Some(field) => {
                field.backspace();
                DialogOutcome::Handled
            }
            None => DialogOutcome::Ignored,
        },
        // Swallowed even with one field, so `Tab` never moves the shell's focus zones behind
        // the scrim.
        DialogKey::Tab => {
            dialog.cycle_field();
            DialogOutcome::Handled
        }
        DialogKey::Enter if dialog.is_valid() => DialogOutcome::Confirm,
        DialogKey::Enter => DialogOutcome::Handled,
        DialogKey::Char(ch) => match dialog.focused_text() {
            Some(field) => {
                field.push(ch);
                DialogOutcome::Handled
            }
            None => DialogOutcome::Ignored,
        },
        DialogKey::Other => DialogOutcome::Ignored,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{
        AccountType, AddInstitutionForm, AddUnitField, DeleteUnitForm, UnitForm, default_units,
    };

    fn type_text(dialog: &mut OpenDialog, text: &str) {
        for ch in text.chars() {
            assert_eq!(
                handle_key(dialog, DialogKey::Char(ch)),
                DialogOutcome::Handled
            );
        }
    }

    fn unit_form(dialog: &OpenDialog) -> &UnitForm {
        match dialog {
            OpenDialog::Settings(SettingsDialog::AddUnit(form)) => form,
            other => panic!("expected Add unit, got {other:?}"),
        }
    }

    #[test]
    fn typing_and_backspace_edit_the_focused_field_only() {
        let mut dialog = OpenDialog::Settings(SettingsDialog::AddUnit(UnitForm::default()));
        type_text(&mut dialog, "aud");
        handle_key(&mut dialog, DialogKey::Backspace);
        assert_eq!(unit_form(&dialog).code.text(), "au");

        handle_key(&mut dialog, DialogKey::Tab);
        assert_eq!(unit_form(&dialog).focused_field, AddUnitField::Name);
        type_text(&mut dialog, "x");
        assert_eq!(unit_form(&dialog).name.text(), "x");
        assert_eq!(unit_form(&dialog).code.text(), "au");
    }

    #[test]
    fn enter_confirms_only_once_the_form_is_valid() {
        let mut dialog = OpenDialog::Settings(SettingsDialog::AddUnit(UnitForm::default()));
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        type_text(&mut dialog, "aud");
        handle_key(&mut dialog, DialogKey::Tab);
        type_text(&mut dialog, "Australian dollar");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    #[test]
    fn delete_unit_confirms_only_on_the_exact_case_sensitive_code() {
        let mut dialog =
            OpenDialog::Settings(SettingsDialog::DeleteUnit(0, DeleteUnitForm::new("btc")));
        type_text(&mut dialog, "BTC");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Handled
        );
        for _ in 0..3 {
            handle_key(&mut dialog, DialogKey::Backspace);
        }
        type_text(&mut dialog, "btc");
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    #[test]
    fn add_institution_needs_a_name_an_account_type_and_a_unit() {
        let mut dialog = OpenDialog::Settings(SettingsDialog::AddInstitution(
            AddInstitutionForm::new(&default_units()),
        ));
        assert!(!dialog.is_valid());
        type_text(&mut dialog, "ANZ");
        assert!(dialog.is_valid());
        if let OpenDialog::Settings(SettingsDialog::AddInstitution(form)) = &mut dialog {
            form.toggle_account_type(AccountType::Savings);
        }
        assert!(!dialog.is_valid());
    }

    #[test]
    fn a_dialog_without_text_fields_ignores_typing_but_swallows_tab() {
        let mut dialog = OpenDialog::Settings(SettingsDialog::ClearLogs);
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Char('x')),
            DialogOutcome::Ignored
        );
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Backspace),
            DialogOutcome::Ignored
        );
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Tab),
            DialogOutcome::Handled
        );
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Enter),
            DialogOutcome::Confirm
        );
    }

    #[test]
    fn other_keys_fall_through() {
        let mut dialog = OpenDialog::Settings(SettingsDialog::AddUnit(UnitForm::default()));
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Other),
            DialogOutcome::Ignored
        );
    }
}
