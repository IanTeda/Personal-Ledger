//! The Accounts Dialogs' confirm cycle (ADR-0037): the page command each form describes, a
//! `lib_ledger` command in each variant, confirmed through [`confirm_ledger`].

use chrono::Local;
use gpui::App;
use lib_accounts::{AccountDraft, AccountError};
use lib_core::AccountId;
use lib_ledger::{AddAccount, DeleteAccount, EditAccount, Ledger, LedgerCommand};
use lib_toast::ToastKind;

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
    /// Delete the account and the Transactions booked to it.
    Delete(DeleteAccount),
}

/// What a landed [`AccountCommand`] did, for the selection and Toast that follow it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountOutcome {
    /// Added or edited: the account to select.
    Saved(AccountId),
    /// Deleted: the name, and how many Transactions went with it.
    Deleted { name: String, transactions: usize },
}

/// A delete raises the Toast naming the account and how many Transactions went with it.
impl OutcomeToast for AccountOutcome {
    fn toast(&self) -> Option<DialogToast> {
        match self {
            Self::Saved(_) => None,
            Self::Deleted { name, transactions } => Some((
                ToastKind::Success,
                lib_locale::msg::toast_account_deleted(
                    name,
                    i64::try_from(*transactions).unwrap_or(i64::MAX),
                ),
            )),
        }
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
            Self::Delete(delete) => {
                let deleted = delete.run(ledger)?;
                Ok(AccountOutcome::Deleted {
                    name: deleted.name,
                    transactions: deleted.transactions,
                })
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
            Self::Delete(id, form) if form.is_valid() => {
                Ok(AccountCommand::Delete(DeleteAccount(*id)))
            }
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
        settings::accounts::form::{AccountOptions, DeleteAccountForm},
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

    fn first_account(ledger: &Entity<LedgerStore>, cx: &TestAppContext) -> (AccountId, String) {
        ledger.read_with(cx, |store, _| {
            let account = &store.ledger().accounts()[0];
            (account.id, account.name.clone())
        })
    }

    fn account_count(ledger: &Entity<LedgerStore>, cx: &TestAppContext) -> usize {
        ledger.read_with(cx, |store, _| store.ledger().accounts().len())
    }

    fn add_dialog(ledger: &Entity<LedgerStore>, cx: &TestAppContext) -> AccountsDialog {
        let options = ledger.read_with(cx, |store, _| {
            AccountOptions::new(
                vec!["ANZ Banking Group".to_string()],
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
    fn a_delete_not_yet_confirmed_by_name_stays_open_and_deletes_nothing(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let before = account_count(&ledger, cx);
        let (id, name) = first_account(&ledger, cx);
        let mut dialog = AccountsDialog::Delete(id, DeleteAccountForm::new(name));

        let landed = cx.update(|cx| confirm_ledger(&mut dialog, &ledger, cx));

        assert!(landed.is_none());
        assert_eq!(account_count(&ledger, cx), before);
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
    fn a_delete_of_a_vanished_account_is_refused_and_stays_open(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let before = account_count(&ledger, cx);
        let mut dialog = AccountsDialog::Delete(AccountId::new(), DeleteAccountForm::new("Gone"));
        if let AccountsDialog::Delete(_, form) = &mut dialog {
            form.confirm_input = TextField::new("Gone");
        }

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
        let AccountOutcome::Saved(id) = landed.outcome else {
            panic!("an add saves, got {:?}", landed.outcome);
        };
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

    #[gpui::test]
    fn a_confirmed_delete_removes_the_account_and_toasts_its_name(cx: &mut TestAppContext) {
        let ledger = ledger(cx);
        let (id, name) = first_account(&ledger, cx);
        let mut dialog = AccountsDialog::Delete(id, DeleteAccountForm::new(name.as_str()));
        if let AccountsDialog::Delete(_, form) = &mut dialog {
            form.confirm_input = TextField::new(name.as_str());
        }

        let landed = cx
            .update(|cx| confirm_ledger(&mut dialog, &ledger, cx))
            .expect("a name typed back deletes");

        assert_eq!(
            landed.outcome,
            AccountOutcome::Deleted {
                name: name.clone(),
                transactions: 0
            }
        );
        assert_eq!(
            landed.toast,
            Some((
                ToastKind::Success,
                lib_locale::msg::toast_account_deleted(&name, 0)
            ))
        );
        ledger.read_with(cx, |store, _| {
            assert!(store.ledger().accounts().iter().all(|a| a.id != id));
        });
    }
}
