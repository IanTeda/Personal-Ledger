//! The Accounts writes (ADR-0038): each a command value with its own refusal, checked before
//! anything is written so a refusal leaves the Ledger untouched, and recording an upsert or delete
//! per row it wrote (ADR-0039).
//!
//! Single-Account rules stay in `lib_accounts`. What lives here crosses domains: deleting an
//! Account deletes the Transactions booked to it.

use lib_accounts::{
    Account, AccountDraft, AccountError, NewAccount, count_imported, edit_account, get,
    insert_account, remove_account,
};
use lib_core::AccountId;

use crate::{Ledger, LedgerChange, LedgerCommand};

/// What deleting an Account removed, for the Toast that follows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountDeleted {
    pub name: String,
    /// How many Transactions went with it.
    pub transactions: usize,
}

impl Ledger {
    /// Adds the Account `new` describes and returns its id.
    pub fn add_account(&mut self, new: NewAccount) -> AccountId {
        let id = insert_account(&mut self.accounts, new);
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

    /// Deletes Account `id` and the Transactions booked to it, as the Delete Dialog says.
    pub fn delete_account(&mut self, id: AccountId) -> Result<AccountDeleted, AccountError> {
        let removed = remove_account(&mut self.accounts, id)?;
        self.pending.push(LedgerChange::DeleteAccount(id));
        let mut transactions = 0;
        self.transactions.retain(|transaction| {
            let booked = transaction.account_id == id;
            if booked {
                transactions += 1;
                self.pending
                    .push(LedgerChange::DeleteTransaction(transaction.id));
            }
            !booked
        });
        Ok(AccountDeleted {
            name: removed.name,
            transactions,
        })
    }

    /// Adds `imported` to Account `id`'s stub Transaction count, which the Edit and Delete Dialogs
    /// show, after an Import books that many Transactions to it.
    pub fn count_imported_transactions(
        &mut self,
        id: AccountId,
        imported: u32,
    ) -> Result<(), AccountError> {
        count_imported(&mut self.accounts, id, imported)?;
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

/// [`Ledger::delete_account`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteAccount(pub AccountId);

impl LedgerCommand for DeleteAccount {
    type Outcome = AccountDeleted;
    type Refusal = AccountError;
    fn run(self, ledger: &mut Ledger) -> Result<AccountDeleted, AccountError> {
        ledger.delete_account(self.0)
    }
}

/// [`Ledger::count_imported_transactions`] as a value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CountImportedTransactions {
    pub id: AccountId,
    pub imported: u32,
}

impl LedgerCommand for CountImportedTransactions {
    type Outcome = ();
    type Refusal = AccountError;
    fn run(self, ledger: &mut Ledger) -> Result<(), AccountError> {
        ledger.count_imported_transactions(self.id, self.imported)
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
        assert_eq!(
            (account.name.as_str(), account.transaction_count),
            ("Fresh", 0)
        );
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
    fn a_refused_edit_or_delete_leaves_the_ledger_untouched() {
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
        assert_eq!(
            DeleteAccount(AccountId::seed(999)).run(&mut ledger),
            Err(AccountError::NotFound)
        );
        assert_eq!(ledger.accounts(), accounts);
        assert_eq!(ledger.transactions(), transactions);
        assert!(ledger.drain_changes().is_empty());
    }

    #[test]
    fn delete_removes_the_account_and_records_each_transaction_that_went_with_it() {
        let mut ledger = Ledger::stub();
        let account = ledger
            .accounts()
            .iter()
            .find(|account| {
                ledger
                    .transactions()
                    .iter()
                    .any(|t| t.account_id == account.id)
            })
            .expect("the seed books Transactions to an Account")
            .clone();
        let booked = ledger
            .transactions()
            .iter()
            .filter(|t| t.account_id == account.id)
            .count();
        let total = ledger.transactions().len();

        let deleted = DeleteAccount(account.id).run(&mut ledger).unwrap();

        assert_eq!(
            deleted,
            AccountDeleted {
                name: account.name,
                transactions: booked,
            }
        );
        assert!(get(ledger.accounts(), account.id).is_none());
        assert_eq!(ledger.transactions().len(), total - booked);
        assert!(
            ledger
                .transactions()
                .iter()
                .all(|t| t.account_id != account.id)
        );
        let changes = ledger.drain_changes();
        assert_eq!(changes.len(), 1 + booked);
        assert_eq!(changes[0], LedgerChange::DeleteAccount(account.id));
        assert!(
            changes[1..]
                .iter()
                .all(|change| matches!(change, LedgerChange::DeleteTransaction(_)))
        );
    }

    #[test]
    fn counting_an_import_adds_to_the_stub_count_and_a_missing_account_is_refused() {
        let mut ledger = Ledger::stub();
        let before = ledger.accounts()[0].clone();

        CountImportedTransactions {
            id: before.id,
            imported: 3,
        }
        .run(&mut ledger)
        .unwrap();

        assert_eq!(
            get(ledger.accounts(), before.id).unwrap().transaction_count,
            before.transaction_count + 3
        );
        assert_eq!(ledger.drain_changes().len(), 1);
        assert_eq!(
            CountImportedTransactions {
                id: AccountId::seed(999),
                imported: 1,
            }
            .run(&mut ledger),
            Err(AccountError::NotFound)
        );
        assert!(ledger.drain_changes().is_empty());
    }
}
