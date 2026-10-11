//! The Accounts Dialogs' confirm cycle (ADR-0037): the command each form describes and the
//! [`Apply`] that runs it against the Accounts, Units and Transactions stores. When `LedgerStore`
//! replaces the per-domain stores only [`AccountsApply`] changes.

use chrono::Local;
use gpui::{App, Entity};

use super::form::{AccountForm, AccountsDialog};
use crate::{
    chrome::dialog_apply::{Apply, DialogForm, FormError},
    transactions::{TransactionsStore, edit_transactions},
    units::UnitsStore,
    view::accounts::state::AccountsStore,
};

/// What confirming an Accounts Dialog asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountCommand {
    Add(AccountForm),
    Edit(u32, AccountForm),
    /// Delete the account and the Transactions booked to it.
    Delete(u32),
}

/// What a landed [`AccountCommand`] did, for the selection and Toast that follow it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountOutcome {
    /// Added or edited: the account to select.
    Saved(u32),
    /// Deleted: the name, read before the write removes it, and how many Transactions went with it.
    Deleted { name: String, transactions: usize },
}

/// Why an account command didn't land. The forms validate first, so this only happens when the
/// account was deleted while its Dialog was open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountRefusal {
    NotFound,
    Invalid,
}

impl DialogForm for AccountsDialog {
    type Command = AccountCommand;
    type Refusal = AccountRefusal;

    fn command(&self) -> Result<AccountCommand, FormError> {
        match self {
            Self::Add(form) if form.is_valid() => Ok(AccountCommand::Add(form.clone())),
            Self::Edit(id, form) if form.is_valid() => Ok(AccountCommand::Edit(*id, form.clone())),
            Self::Delete(id, form) if form.is_valid() => Ok(AccountCommand::Delete(*id)),
            _ => Err(FormError),
        }
    }

    /// A refusal here means the account vanished under an open, valid form, so there is nothing
    /// left for the form to correct: the Dialog stays open and `Esc` closes it.
    fn refuse(&mut self, _refusal: AccountRefusal) {}
}

/// Runs [`AccountCommand`]s against the Accounts store, reading the Unit an account is held in
/// from the Units store and deleting its Transactions from theirs.
pub struct AccountsApply {
    pub accounts: Entity<AccountsStore>,
    pub units: Entity<UnitsStore>,
    pub transactions: Entity<TransactionsStore>,
}

impl Apply<AccountCommand> for AccountsApply {
    type Outcome = AccountOutcome;
    type Refusal = AccountRefusal;

    fn apply(
        &mut self,
        command: AccountCommand,
        cx: &mut App,
    ) -> Result<AccountOutcome, AccountRefusal> {
        match command {
            AccountCommand::Add(form) => {
                let is_currency = form
                    .unit
                    .value()
                    .and_then(|code| {
                        self.units
                            .read(cx)
                            .units()
                            .iter()
                            .find(|unit| unit.code == code)
                    })
                    .is_none_or(|unit| unit.kind == "currency");
                let opened_at = Local::now().date_naive();
                self.accounts
                    .update(cx, |store, cx| {
                        store.mutate(cx, |service| {
                            let id = service.next_id();
                            let account = form.into_account(id, opened_at, is_currency)?;
                            service.insert(account);
                            Some(id)
                        })
                    })
                    .map(AccountOutcome::Saved)
                    .ok_or(AccountRefusal::Invalid)
            }
            AccountCommand::Edit(id, form) => self.accounts.update(cx, |store, cx| {
                store.mutate(cx, |service| {
                    let account = service.get_mut(id).ok_or(AccountRefusal::NotFound)?;
                    form.apply_to(account)
                        .then_some(AccountOutcome::Saved(id))
                        .ok_or(AccountRefusal::Invalid)
                })
            }),
            AccountCommand::Delete(id) => {
                let name = self
                    .accounts
                    .read(cx)
                    .accounts()
                    .iter()
                    .find(|account| account.id == id)
                    .map(|account| account.name.clone())
                    .ok_or(AccountRefusal::NotFound)?;
                let transactions = edit_transactions(&self.transactions, cx, |transactions| {
                    let before = transactions.len();
                    transactions.retain(|transaction| transaction.account_id != id);
                    before - transactions.len()
                });
                self.accounts.update(cx, |store, cx| {
                    store.mutate(cx, |service| service.remove(id))
                });
                Ok(AccountOutcome::Deleted { name, transactions })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext, TestAppContext};

    use super::*;
    use crate::{accounts::form::DeleteAccountForm, chrome::dialog_apply::confirm};

    #[gpui::test]
    fn a_confirmed_delete_removes_the_account_and_reports_its_name(cx: &mut TestAppContext) {
        let accounts = cx.new(|_| AccountsStore::seeded());
        let units = cx.new(|_| {
            UnitsStore::new(
                crate::units::default_units(),
                crate::units::default_price_sources(),
            )
        });
        let transactions = cx.new(|_| TransactionsStore::new(Vec::new()));
        let mut apply = AccountsApply {
            accounts: accounts.clone(),
            units,
            transactions,
        };
        let (id, name) = cx.read(|cx| {
            let account = &accounts.read(cx).accounts()[0];
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
        cx.read(|cx| assert!(accounts.read(cx).accounts().iter().all(|a| a.id != id)));
    }
}
