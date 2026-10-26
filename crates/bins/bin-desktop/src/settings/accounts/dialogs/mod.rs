//! The Accounts page's Dialogs: Add, Edit and Delete account. [`AccountsDialog`] is the value the
//! Dialog host holds while one is open (`OpenDialog::Accounts`). It draws itself through `add`,
//! `edit` and `delete` (`Dialog::render`), every click reaching the form through the host's one
//! edit callback, reads itself back for the UI tests and confirms itself through `apply.rs`.

pub mod add;
pub mod delete;
pub mod edit;

use std::rc::Rc;

use gpui::{AnyElement, App, Window, div, prelude::*};
use lib_core::AccountId;

use super::{
    dialog_options,
    form::{AccountField, AccountForm, DeleteAccountForm},
};
use crate::{
    form::field::TextField,
    shell::view::dialog_host::{
        Dialog, DialogActions, DialogKey, DialogOutcome, DialogReadback, DialogScope,
    },
};

/// The dialogs' shared width: `docs/ux/desktop-mockups/17-accounts/README.md`'s "Dialog" row.
pub const WIDTH: gpui::Pixels = gpui::px(440.0);

/// Every Accounts dialog `Root` can have open, `None` when none is -- the README's own `State`
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

    fn render(&self, scope: &DialogScope, actions: &DialogActions, cx: &App) -> Option<AnyElement> {
        let on_field_click: add::OnFieldClick = {
            let actions = actions.clone();
            Rc::new(move |field, _window: &mut Window, cx| {
                actions.edit(cx, |dialog: &mut Self, _| {
                    dialog.click_field(field);
                    None
                });
            })
        };
        let on_option_click: add::OnOptionClick = {
            let actions = actions.clone();
            Rc::new(move |field, index, _window: &mut Window, cx| {
                actions.edit(cx, |dialog: &mut Self, _| {
                    dialog.choose_option(field, index);
                    None
                });
            })
        };
        let ledger = scope.ledger.read(cx).ledger();
        let account = |id: &AccountId| ledger.accounts().iter().find(|a| a.id == *id);
        Some(match self {
            Self::Add(form) => add::render(
                form,
                &dialog_options(&scope.ledger, None, cx),
                on_field_click,
                on_option_click,
                actions.cancel.clone(),
                actions.confirm.clone(),
                cx,
            ),
            Self::Edit(id, form) => match account(id) {
                Some(account) => edit::render(
                    form,
                    account,
                    scope.ledger.read(cx).account_usage(account.id),
                    &dialog_options(&scope.ledger, Some(account.unit), cx),
                    on_field_click,
                    on_option_click,
                    actions.cancel.clone(),
                    actions.confirm.clone(),
                    cx,
                ),
                // Defensive only: the id comes from a live row when the dialog opens.
                None => div().into_any_element(),
            },
            Self::Delete(id, form) => match account(id) {
                Some(account) => delete::render(
                    account,
                    scope.ledger.read(cx).account_usage(account.id),
                    ledger.unit_code(account.unit),
                    form,
                    actions.cancel.clone(),
                    actions.confirm.clone(),
                    cx,
                ),
                // Defensive only: the id comes from a live row when the dialog opens.
                None => div().into_any_element(),
            },
        })
    }

    /// Reads back an Add or Edit form's focused field (its `AccountField` variant name), drafts,
    /// selects and whether it would confirm; a Delete's typed-back `confirm`. Edit and Delete also
    /// read back the `transactions` and `budgets` counts they show.
    fn readback(&self, scope: &DialogScope, cx: &App) -> Option<DialogReadback> {
        let usage = |id: &AccountId| {
            let usage = scope.ledger.read(cx).account_usage(*id);
            [
                ("transactions", usage.transactions.to_string()),
                ("budgets", usage.budgets.to_string()),
            ]
        };
        let (kind, form, counts) = match self {
            Self::Add(form) => ("AddAccount", form, None),
            Self::Edit(id, form) => ("EditAccount", form, Some(usage(id))),
            Self::Delete(id, form) => {
                let mut values = vec![("confirm", form.confirm_input.text().to_string())];
                values.extend(usage(id));
                return Some(DialogReadback {
                    kind: "DeleteAccount",
                    values,
                });
            }
        };
        let mut readback = DialogReadback {
            kind,
            values: vec![
                ("focused", format!("{:?}", form.focused)),
                ("name", form.name.text().to_string()),
                (
                    "institution",
                    form.institution.value().unwrap_or_default().to_string(),
                ),
                (
                    "type",
                    form.account_type.value().unwrap_or_default().to_string(),
                ),
                ("unit", form.unit.value().unwrap_or_default().to_string()),
                ("unit_options", form.unit_choices().join("|")),
                ("opening_balance", form.opening_balance.text().to_string()),
                ("account_number", form.account_number.text().to_string()),
                ("select_open", form.any_select_open().to_string()),
                ("valid", form.is_valid().to_string()),
            ],
        };
        readback.values.extend(counts.into_iter().flatten());
        Some(readback)
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
