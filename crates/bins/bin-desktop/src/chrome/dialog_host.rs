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

use crate::{
    bills::BillsDialog, budgets::BudgetsDialog, categories::form::CategoriesDialog,
    documents::form::DocumentsDialog, documents::types::DocumentTypesDialog,
    form::field::TextField, inventory::form::InventoryDialog, payees::form::PayeesDialog,
    settings::SettingsDialog, settings::accounts::dialogs::AccountsDialog,
    settings::institutions::dialogs::InstitutionsDialog, settings::units::dialogs::UnitsDialog,
    tags::form::TagsDialog,
};

/// A keystroke as a Dialog sees it, already stripped of modifiers by `Shell`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogKey {
    Backspace,
    Tab,
    /// `Shift-Tab`; a Dialog without its own handling treats it as `Tab`.
    BackTab,
    Up,
    Down,
    Left,
    Right,
    Enter,
    /// `Ctrl-Enter`: a multi-line box takes it to save, and any other Dialog treats it as `Enter`.
    CtrlEnter,
    /// A printable character, typed without Ctrl/Alt/Cmd/Fn.
    Char(char),
    /// A character chord held with Ctrl alone (`Ctrl-N`), for a list Dialog's own keys.
    Ctrl(char),
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

    /// The status line's hints while this Dialog is open, in place of the View's (ADR-0037).
    /// `None` leaves them to the View, as for a Dialog whose View still words its hints itself.
    fn hints(&self) -> Option<Vec<(&'static str, String)>> {
        None
    }
}

/// The session Toast history: read-only, so it takes no key but `Esc`, which the router turns
/// into closing it. Showing Toasts hide behind it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToastHistoryDialog;

impl Dialog for ToastHistoryDialog {
    /// The list scrolls by pointer, so every key is left to the shell.
    fn handle_own_key(&mut self, _key: DialogKey) -> Option<DialogOutcome> {
        Some(DialogOutcome::Ignored)
    }
}

/// The open Dialog, one variant per feature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenDialog {
    Settings(SettingsDialog),
    Units(UnitsDialog),
    Institutions(InstitutionsDialog),
    Accounts(AccountsDialog),
    Categories(CategoriesDialog),
    Payees(PayeesDialog),
    DocumentTypes(DocumentTypesDialog),
    Inventory(InventoryDialog),
    Tags(TagsDialog),
    /// Boxed: the plan form carries its select choices, which would swell every variant.
    Bills(Box<BillsDialog>),
    /// Boxed: its forms carry their select choices.
    Budgets(Box<BudgetsDialog>),
    Documents(DocumentsDialog),
    ToastHistory(ToastHistoryDialog),
}

impl OpenDialog {
    fn inner(&self) -> &dyn Dialog {
        match self {
            Self::Settings(dialog) => dialog,
            Self::Units(dialog) => dialog,
            Self::Institutions(dialog) => dialog,
            Self::Accounts(dialog) => dialog,
            Self::Categories(dialog) => dialog,
            Self::Payees(dialog) => dialog,
            Self::DocumentTypes(dialog) => dialog,
            Self::Inventory(dialog) => dialog,
            Self::Tags(dialog) => dialog,
            Self::Bills(dialog) => &**dialog,
            Self::Budgets(dialog) => &**dialog,
            Self::Documents(dialog) => dialog,
            Self::ToastHistory(dialog) => dialog,
        }
    }

    fn inner_mut(&mut self) -> &mut dyn Dialog {
        match self {
            Self::Settings(dialog) => dialog,
            Self::Units(dialog) => dialog,
            Self::Institutions(dialog) => dialog,
            Self::Accounts(dialog) => dialog,
            Self::Categories(dialog) => dialog,
            Self::Payees(dialog) => dialog,
            Self::DocumentTypes(dialog) => dialog,
            Self::Inventory(dialog) => dialog,
            Self::Tags(dialog) => dialog,
            Self::Bills(dialog) => &mut **dialog,
            Self::Budgets(dialog) => &mut **dialog,
            Self::Documents(dialog) => dialog,
            Self::ToastHistory(dialog) => dialog,
        }
    }
}

/// A feature's Dialog as one [`OpenDialog`] variant holds it, so `Shell::dialog_mut::<T>()` finds
/// the open Dialog by type instead of one accessor per feature.
pub trait DialogVariant: Sized {
    fn from_open(open: &OpenDialog) -> Option<&Self>;
    fn from_open_mut(open: &mut OpenDialog) -> Option<&mut Self>;
}

/// Generates the [`DialogVariant`] impl for the `OpenDialog` variant that holds `$ty`; a boxed
/// variant passes the inner type and the box derefs.
macro_rules! dialog_variant {
    ($variant:ident, $ty:ty) => {
        impl DialogVariant for $ty {
            fn from_open(open: &OpenDialog) -> Option<&Self> {
                match open {
                    OpenDialog::$variant(dialog) => Some(dialog),
                    _ => None,
                }
            }

            fn from_open_mut(open: &mut OpenDialog) -> Option<&mut Self> {
                match open {
                    OpenDialog::$variant(dialog) => Some(dialog),
                    _ => None,
                }
            }
        }
    };
}

dialog_variant!(Settings, SettingsDialog);
dialog_variant!(Units, UnitsDialog);
dialog_variant!(Institutions, InstitutionsDialog);
dialog_variant!(Accounts, AccountsDialog);
dialog_variant!(Categories, CategoriesDialog);
dialog_variant!(Payees, PayeesDialog);
dialog_variant!(DocumentTypes, DocumentTypesDialog);
dialog_variant!(Inventory, InventoryDialog);
dialog_variant!(Tags, TagsDialog);
dialog_variant!(Bills, BillsDialog);
dialog_variant!(Budgets, BudgetsDialog);
dialog_variant!(Documents, DocumentsDialog);

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

    fn hints(&self) -> Option<Vec<(&'static str, String)>> {
        self.inner().hints()
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
        DialogKey::Tab | DialogKey::BackTab => {
            dialog.cycle_field();
            DialogOutcome::Handled
        }
        DialogKey::Enter | DialogKey::CtrlEnter if dialog.is_valid() => DialogOutcome::Confirm,
        DialogKey::Enter | DialogKey::CtrlEnter => DialogOutcome::Handled,
        DialogKey::Char(ch) => match dialog.focused_text() {
            Some(field) => {
                field.push(ch);
                DialogOutcome::Handled
            }
            None => DialogOutcome::Ignored,
        },
        DialogKey::Up
        | DialogKey::Down
        | DialogKey::Left
        | DialogKey::Right
        | DialogKey::Ctrl(_)
        | DialogKey::Other => DialogOutcome::Ignored,
    }
}

/// Types `text` into whichever field the Dialog has focused, for the form tests that sit beside
/// each form.
#[cfg(test)]
pub(crate) fn type_text(dialog: &mut impl Dialog, text: &str) {
    for ch in text.chars() {
        assert_eq!(
            handle_key(dialog, DialogKey::Char(ch)),
            DialogOutcome::Handled
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::settings::units::form::UnitForm;

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
        let mut dialog = OpenDialog::Units(UnitsDialog::Add(UnitForm::default()));
        assert_eq!(
            handle_key(&mut dialog, DialogKey::Other),
            DialogOutcome::Ignored
        );
    }

    #[test]
    fn dialog_variant_finds_only_its_own_features_dialog() {
        let mut dialog = OpenDialog::Settings(SettingsDialog::ClearLogs);
        assert!(SettingsDialog::from_open(&dialog).is_some());
        assert!(SettingsDialog::from_open_mut(&mut dialog).is_some());
        assert!(AccountsDialog::from_open(&dialog).is_none());
        assert!(AccountsDialog::from_open_mut(&mut dialog).is_none());
    }

    #[test]
    fn toast_history_ignores_every_key_and_confirms_nothing_by_typing() {
        let mut dialog = OpenDialog::ToastHistory(ToastHistoryDialog);
        for key in [
            DialogKey::Char('x'),
            DialogKey::Backspace,
            DialogKey::Tab,
            DialogKey::BackTab,
            DialogKey::Enter,
            DialogKey::Down,
        ] {
            assert_eq!(handle_key(&mut dialog, key), DialogOutcome::Ignored);
        }
        assert!(!dialog.close_open_select(), "the first Esc closes it");
    }
}
