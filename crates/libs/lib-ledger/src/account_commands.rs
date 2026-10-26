//! The Accounts writes (ADR-0038): each a command value with its own refusal, checked before
//! anything is written so a refusal leaves the Ledger untouched, and recording an upsert or delete
//! per row it wrote (ADR-0039).
//!
//! Single-Account rules stay in `lib_accounts`. Deleting an Account, which crosses domains, is
//! `lib-account-deletion`'s (#854).

use lib_accounts::{
    Account, AccountDraft, AccountError, NewAccount, edit_account, get, insert_account,
};
use lib_core::AccountId;

use crate::{Ledger, LedgerChange, LedgerCommand};

impl Ledger {
    /// Adds the Account `new` describes and returns its id.
    pub fn add_account(&mut self, new: NewAccount) -> AccountId {
        let id = insert_account(&mut self.accounts, new).id;
        self.record_account(id);
        id
    }

    /// Saves the editable fields of Account `id`.
    pub fn edit_account(
        &mut self,
        id: AccountId,
        draft: &AccountDraft,
    ) -> Result<(), AccountError> {
        edit_account(&mut self.accounts, id, draft)?;
        self.record_account(id);
        Ok(())
    }

    fn record_account(&mut self, id: AccountId) {
        if let Some(account) = get(&self.accounts, id) {
            self.pending
                .push(LedgerChange::UpsertAccount(Account::clone(account)));
        }
    }
}

/// [`Ledger::add_account`] as a value.
#[derive(Debug, Clone, PartialEq)]
pub struct AddAccount(pub NewAccount);

impl LedgerCommand for AddAccount {
    type Outcome = AccountId;
    type Refusal = AccountError;
    fn run(self, ledger: &mut Ledger) -> Result<AccountId, AccountError> {
        Ok(ledger.add_account(self.0))
    }
}

/// [`Ledger::edit_account`] as a value.
#[derive(Debug, Clone, PartialEq)]
pub struct EditAccount {
    pub id: AccountId,
    pub draft: AccountDraft,
}

impl LedgerCommand for EditAccount {
    type Outcome = ();
    type Refusal = AccountError;
    fn run(self, ledger: &mut Ledger) -> Result<(), AccountError> {
        ledger.edit_account(self.id, &self.draft)
    }
}

#[cfg(test)]
mod tests {
    use bigdecimal::BigDecimal;
    use chrono::NaiveDate;
    use lib_core::{AccountType, InstitutionId, Money, UnitId};

    use super::*;

    fn draft(name: &str) -> AccountDraft {
        AccountDraft {
            name: name.to_owned(),
            institution: InstitutionId::SEED_ANZ,
            account_type: AccountType::Bank,
            account_number: Some("123".to_owned()),
        }
    }

    fn new_account(name: &str) -> NewAccount {
        NewAccount {
            draft: draft(name),
            unit: UnitId::SEED_AUD,
            balance: Money(BigDecimal::from(10)),
            opened_at: NaiveDate::from_ymd_opt(2026, 1, 1).expect("a valid date"),
        }
    }

    #[test]
    fn add_records_the_new_account_under_a_fresh_id() {
        let mut ledger = Ledger::stub();

        let id = AddAccount(new_account("Fresh")).run(&mut ledger).unwrap();

        assert!(ledger.accounts().iter().filter(|a| a.id == id).count() == 1);
        let account = get(ledger.accounts(), id).unwrap();
        assert_eq!(account.name, "Fresh");
        let changes = ledger.drain_changes();
        assert!(matches!(&changes[..], [LedgerChange::UpsertAccount(a)] if a.id == id));
    }

    #[test]
    fn edit_writes_the_draft_and_keeps_the_fixed_fields() {
        let mut ledger = Ledger::stub();
        let before = ledger.accounts()[0].clone();

        EditAccount {
            id: before.id,
            draft: draft("Renamed"),
        }
        .run(&mut ledger)
        .unwrap();

        let after = get(ledger.accounts(), before.id).unwrap();
        assert_eq!(after.name, "Renamed");
        assert_eq!(after.account_number.as_deref(), Some("123"));
        assert_eq!(
            (&after.unit, &after.balance, after.opened_at),
            (&before.unit, &before.balance, before.opened_at)
        );
        assert_eq!(ledger.drain_changes().len(), 1);
    }

    #[test]
    fn a_refused_edit_leaves_the_ledger_untouched() {
        let mut ledger = Ledger::stub();
        let accounts = ledger.accounts().to_vec();
        let transactions = ledger.transactions().to_vec();

        assert_eq!(
            EditAccount {
                id: AccountId::seed(999),
                draft: draft("x"),
            }
            .run(&mut ledger),
            Err(AccountError::NotFound)
        );
        assert_eq!(ledger.accounts(), accounts);
        assert_eq!(ledger.transactions(), transactions);
        assert!(ledger.drain_changes().is_empty());
    }
}
