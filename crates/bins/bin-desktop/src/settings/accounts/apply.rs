//! The Add and Edit account Dialogs' confirm cycle (ADR-0037): the page command each form
//! describes, a `lib_ledger` command in each variant, confirmed through [`confirm_ledger`]. Delete
//! is `lib-account-deletion`'s Dialog (#854).

use chrono::Local;
use gpui::App;
use lib_accounts::{AccountDraft, AccountError};
use lib_core::AccountId;
use lib_ledger::{AddAccount, EditAccount, Ledger, LedgerCommand};

use super::{dialogs::AccountsDialog, form::AccountForm};
use crate::shell::view::{
    dialog_apply::{DialogForm, FormError, OutcomeToast, confirm_ledger},
    dialog_host::{ConfirmDialog, DialogScope, DialogToast, Landed},
};

/// What confirming an Accounts Dialog asks the Ledger for.
#[derive(Debug, Clone, PartialEq)]
pub enum AccountCommand {
    /// Add the account. The Unit's kind decides how the opening balance is padded, which the form
    /// can't know, so the padding happens in `run`, where the Ledger's Units are in reach.
    Add(Box<AccountForm>),
    Edit(EditAccount),
}

/// What a landed [`AccountCommand`] did, for the selection that follows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountOutcome {
    /// Added or edited: the account to select.
    Saved(AccountId),
}

/// An add or an edit raises no Toast.
impl OutcomeToast for AccountOutcome {
    fn toast(&self) -> Option<DialogToast> {
        None
    }
}

impl LedgerCommand for AccountCommand {
    type Outcome = AccountOutcome;
    type Refusal = AccountError;

    fn run(self, ledger: &mut Ledger) -> Result<AccountOutcome, AccountError> {
        match self {
            Self::Add(form) => {
                let is_currency = form
                    .unit
                    .value()
                    .and_then(|code| ledger.units().iter().find(|unit| unit.code == code))
                    .is_none_or(|unit| unit.kind == lib_core::UnitKind::Currency);
                let new = (*form)
                    .into_new_account(Local::now().date_naive(), is_currency)
                    .ok_or(AccountError::Incomplete)?;
                AddAccount(new).run(ledger).map(AccountOutcome::Saved)
            }
            Self::Edit(edit) => {
                let id = edit.id;
                edit.run(ledger).map(|()| AccountOutcome::Saved(id))
            }
        }
    }
}

impl DialogForm for AccountsDialog {
    type Command = AccountCommand;
    type Refusal = AccountError;

    fn command(&self) -> Result<AccountCommand, FormError> {
        match self {
            Self::Add(form) if form.is_valid() => Ok(AccountCommand::Add(Box::new(form.clone()))),
            Self::Edit(id, form) if form.is_valid() => form
                .draft()
                .map(|draft: AccountDraft| AccountCommand::Edit(EditAccount { id: *id, draft }))
                .ok_or(FormError),
            _ => Err(FormError),
        }
    }

    /// A refusal here means the account vanished under an open, valid form, so there is nothing
    /// left for the form to correct: the Dialog stays open and `Esc` closes it.
    fn refuse(&mut self, _refusal: AccountError) {}
}

impl ConfirmDialog for AccountsDialog {
    type Outcome = AccountOutcome;

    fn confirm(&mut self, scope: &DialogScope, cx: &mut App) -> Option<Landed<AccountOutcome>> {
        confirm_ledger(self, &scope.ledger, cx)
    }
}

#[cfg(test)]
mod tests {
    use gpui::{Entity, TestAppContext};
    use lib_ledger::LedgerParts;

    use super::*;
    use crate::{
        form::field::TextField,
        ledger_store::{LedgerStore, test_ledger},
        settings::accounts::form::AccountOptions,
    };

    fn ledger(cx: &mut TestAppContext) -> Entity<LedgerStore> {
        test_ledger(
            cx,
            LedgerParts {
                accounts: lib_accounts::default_accounts(),
                units: lib_units::default_units(chrono::Utc::now()),
                ..LedgerParts::default()
            },
        )
    }

    fn account_count(ledger: &Entity<LedgerStore>, cx: &TestAppContext) -> usize {
        ledger.read_with(cx, |store, _| store.ledger().accounts().len())
    }

    fn add_dialog(ledger: &Entity<LedgerStore>, cx: &TestAppContext) -> AccountsDialog {
        let options = ledger.read_with(cx, |store, _| {
            AccountOptions::new(
                vec![(
                    lib_core::InstitutionId::SEED_ANZ,
                    "ANZ Banking Group".to_string(),
                )],
                store
                    .ledger()
                    .units()
                    .iter()
                    .map(|unit| (unit.id, unit.code.clone()))
                    .collect(),
            )
        });
        AccountsDialog::Add(AccountForm::new(&options, Some("aud")))
    }

    #[gpui::test]
    fn an_add_without_a_name_stays_open_and_adds_nothing(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let before = account_count(&ledger, cx);
        let mut dialog = add_dialog(&ledger, cx);

        let landed = cx.update(|cx| confirm_ledger(&mut dialog, &ledger, cx));

        assert!(landed.is_none());
        assert_eq!(account_count(&ledger, cx), before);
    }

    #[gpui::test]
    fn a_confirmed_add_lands_with_the_new_account_to_select_and_no_toast(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let before = account_count(&ledger, cx);
        let mut dialog = add_dialog(&ledger, cx);
        if let AccountsDialog::Add(form) = &mut dialog {
            form.name = TextField::new("Rainy Day");
            form.opening_balance = TextField::new("1500");
        }

        let landed = cx
            .update(|cx| confirm_ledger(&mut dialog, &ledger, cx))
            .expect("a named account lands");

        assert_eq!(landed.toast, None);
        assert_eq!(account_count(&ledger, cx), before + 1);
        let AccountOutcome::Saved(id) = landed.outcome;
        ledger.read_with(cx, |store, _| {
            let added = store
                .ledger()
                .accounts()
                .iter()
                .find(|account| account.id == id)
                .expect("the selected id is the added row");
            assert_eq!(added.name, "Rainy Day");
        });
    }
}
