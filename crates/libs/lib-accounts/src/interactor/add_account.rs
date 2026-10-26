use crate::protocols::AccountRepository;
use crate::{Account, NewAccount, insert_account};

/// Add the Account a [`NewAccount`] describes, under a fresh id, after every Account already
/// there.
#[derive(Debug, Clone, PartialEq)]
pub struct AddAccount(pub NewAccount);

impl AddAccount {
    /// Returns the Account as written.
    pub fn run(self, accounts: &mut dyn AccountRepository) -> Account {
        let mut rows = accounts.accounts();
        let account = insert_account(&mut rows, self.0);
        accounts.upsert_account(account.clone());
        tracing::info!(account_id = %account.id, "Added an Account");
        account
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use lib_core::{AccountType, InstitutionId, UnitId};

    use super::*;
    use crate::AccountDraft;
    use crate::fake::InMemoryAccounts;

    #[test]
    fn adding_an_account_writes_it_after_the_others_under_a_fresh_id() {
        let mut accounts = InMemoryAccounts::stub();
        let seeded = accounts.accounts().len();

        let added = AddAccount(NewAccount {
            draft: AccountDraft {
                name: "Holiday Fund".into(),
                institution: InstitutionId::SEED_ANZ,
                account_type: AccountType::Bank,
                account_number: None,
            },
            unit: UnitId::SEED_AUD,
            balance: "12.50".parse().expect("a decimal"),
            opened_at: NaiveDate::from_ymd_opt(2026, 10, 1).expect("a date"),
        })
        .run(&mut accounts);

        assert_eq!(accounts.accounts().len(), seeded + 1);
        assert_eq!(accounts.accounts().last(), Some(&added));
        assert_eq!(added.name, "Holiday Fund");
    }
}
