//! The Accounts Dialogs' confirm cycle (ADR-0037): the command each form describes and the
//! [`Apply`] that runs it against the Ledger.

use chrono::Local;
use gpui::{App, Entity};
use lib_accounts::{AccountDraft, AccountError, NewAccount};
use lib_ledger::{AddAccount, DeleteAccount, EditAccount};

use super::form::{AccountForm, AccountsDialog};
use crate::{
    chrome::dialog_apply::{Apply, DialogForm, FormError},
    ledger_store::LedgerStore,
};

/// What confirming an Accounts Dialog asks for.
#[derive(Debug, Clone, PartialEq)]
pub enum AccountCommand {
    /// Add the account. The Unit's kind decides how the opening balance is padded, which the form
    /// can't know, so the padding happens in [`AccountsApply`].
    Add(Box<AccountForm>),
    Edit(u32, AccountDraft),
    /// Delete the account and the Transactions booked to it.
    Delete(u32),
}

/// What a landed [`AccountCommand`] did, for the selection and Toast that follow it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountOutcome {
    /// Added or edited: the account to select.
    Saved(u32),
    /// Deleted: the name, and how many Transactions went with it.
    Deleted { name: String, transactions: usize },
}

impl DialogForm for AccountsDialog {
    type Command = AccountCommand;
    type Refusal = AccountError;

    fn command(&self) -> Result<AccountCommand, FormError> {
        match self {
            Self::Add(form) if form.is_valid() => Ok(AccountCommand::Add(Box::new(form.clone()))),
            Self::Edit(id, form) if form.is_valid() => form
                .draft()
                .map(|draft| AccountCommand::Edit(*id, draft))
                .ok_or(FormError),
            Self::Delete(id, form) if form.is_valid() => Ok(AccountCommand::Delete(*id)),
            _ => Err(FormError),
        }
    }

    /// A refusal here means the account vanished under an open, valid form, so there is nothing
    /// left for the form to correct: the Dialog stays open and `Esc` closes it.
    fn refuse(&mut self, _refusal: AccountError) {}
}

/// Runs [`AccountCommand`]s against the Ledger as Ledger commands. Deleting an account deletes its
/// Transactions in the same command.
pub struct AccountsApply {
    pub ledger: Entity<LedgerStore>,
}

impl Apply<AccountCommand> for AccountsApply {
    type Outcome = AccountOutcome;
    type Refusal = AccountError;

    fn apply(
        &mut self,
        command: AccountCommand,
        cx: &mut App,
    ) -> Result<AccountOutcome, AccountError> {
        match command {
            AccountCommand::Add(form) => {
                let is_currency = form
                    .unit
                    .value()
                    .and_then(|code| {
                        self.ledger
                            .read(cx)
                            .ledger()
                            .units()
                            .iter()
                            .find(|unit| unit.code == code)
                    })
                    .is_none_or(|unit| unit.kind == lib_core::UnitKind::Fiat);
                let new: NewAccount = (*form)
                    .into_new_account(Local::now().date_naive(), is_currency)
                    .ok_or(AccountError::Incomplete)?;
                Ok(AccountOutcome::Saved(
                    self.ledger
                        .update(cx, |store, cx| store.apply(cx, AddAccount(new)))?,
                ))
            }
            AccountCommand::Edit(id, draft) => self
                .ledger
                .update(cx, |store, cx| store.apply(cx, EditAccount { id, draft }))
                .map(|()| AccountOutcome::Saved(id)),
            AccountCommand::Delete(id) => {
                let deleted = self
                    .ledger
                    .update(cx, |store, cx| store.apply(cx, DeleteAccount(id)))?;
                Ok(AccountOutcome::Deleted {
                    name: deleted.name,
                    transactions: deleted.transactions,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;
    use lib_ledger::LedgerParts;

    use super::*;
    use crate::ledger_store::test_ledger;
    use crate::{accounts::form::DeleteAccountForm, chrome::dialog_apply::confirm};

    #[gpui::test]
    fn a_confirmed_delete_removes_the_account_and_reports_its_name(cx: &mut TestAppContext) {
        let ledger = test_ledger(
            cx,
            LedgerParts {
                accounts: crate::accounts::default_accounts(),
                units: lib_units::default_units(),
                price_sources: lib_units::default_price_sources(),
                ..LedgerParts::default()
            },
        );
        let mut apply = AccountsApply {
            ledger: ledger.clone(),
        };
        let (id, name) = cx.read(|cx| {
            let account = &ledger.read(cx).ledger().accounts()[0];
            (account.id, account.name.clone())
        });

        let mut dialog = AccountsDialog::Delete(id, DeleteAccountForm::new(name.as_str()));
        assert_eq!(
            dialog.command(),
            Err(FormError),
            "the name isn't typed back yet"
        );

        if let AccountsDialog::Delete(_, form) = &mut dialog {
            form.confirm_input = crate::form::field::TextField::new(name.as_str());
        }
        let outcome = cx.update(|cx| confirm(&mut dialog, &mut apply, cx));
        assert_eq!(
            outcome,
            Some(AccountOutcome::Deleted {
                name,
                transactions: 0
            })
        );
        cx.read(|cx| {
            assert!(
                ledger
                    .read(cx)
                    .ledger()
                    .accounts()
                    .iter()
                    .all(|a| a.id != id)
            )
        });
    }
}
