//! The Accounts page's Dialogs: Add, Edit and Delete account. [`AccountsDialog`] is the value
//! `Shell`'s Dialog host holds while one is open (`OpenDialog::Accounts`); the Dialog host draws it
//! through `add`, `edit` and `delete`, and confirms it through `apply.rs`.

pub mod add;
pub mod delete;
pub mod edit;

use super::form::{AccountField, AccountForm, DeleteAccountForm};
use crate::{
    chrome::dialog_host::{Dialog, DialogKey, DialogOutcome},
    form::field::TextField,
};
use lib_core::AccountId;

/// The dialogs' shared width: `docs/ux/desktop-mockups/17-accounts/README.md`'s "Dialog" row.
pub const WIDTH: gpui::Pixels = gpui::px(440.0);

/// Every Accounts dialog `Shell` can have open, `None` when none is -- the README's own `State`
/// block (`dialog: Option<Dialog>`). The [`AccountId`] is the account's [`Account::id`]. Each dialog
/// ticket attaches its own form state to its variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountsDialog {
    Add(AccountForm),
    /// Editing the account with this [`Account::id`].
    Edit(AccountId, AccountForm),
    /// Deleting the account with this [`Account::id`], once its name has been typed back.
    Delete(AccountId, DeleteAccountForm),
}

impl AccountsDialog {
    /// The form behind the Add and Edit dialogs; Delete has its own, single-field form.
    pub fn form(&self) -> Option<&AccountForm> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => Some(form),
            Self::Delete(..) => None,
        }
    }

    pub fn form_mut(&mut self) -> Option<&mut AccountForm> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => Some(form),
            Self::Delete(..) => None,
        }
    }
}

impl Dialog for AccountsDialog {
    fn handle_own_key(&mut self, key: DialogKey) -> Option<DialogOutcome> {
        self.form_mut()?.handle_own_key(key)
    }

    fn focused_text(&mut self) -> Option<&mut TextField> {
        match self {
            Self::Add(form) | Self::Edit(_, form) => form.focused_text(),
            Self::Delete(_, form) => Some(&mut form.confirm_input),
        }
    }

    fn cycle_field(&mut self) {
        if let Some(form) = self.form_mut() {
            form.cycle_focus(false);
        }
    }

    fn is_valid(&self) -> bool {
        match self {
            Self::Add(form) | Self::Edit(_, form) => form.is_valid(),
            Self::Delete(_, form) => form.is_valid(),
        }
    }

    fn close_open_select(&mut self) -> bool {
        self.form_mut().is_some_and(AccountForm::close_open_select)
    }
}

impl AccountsDialog {
    /// A click on a field of the Add or Edit form: focuses a text field, or focuses a select and
    /// toggles its list.
    pub fn click_field(&mut self, field: AccountField) {
        if let Some(form) = self.form_mut() {
            if field.is_select() {
                form.click_select(field);
            } else {
                form.focus(field);
            }
        }
    }

    /// A click on a row of an open dropdown list.
    pub fn choose_option(&mut self, field: AccountField, index: usize) {
        if let Some(form) = self.form_mut() {
            form.choose_option(field, index);
        }
    }
}
