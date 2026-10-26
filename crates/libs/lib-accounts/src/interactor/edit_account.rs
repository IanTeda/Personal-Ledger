use lib_core::AccountId;

use crate::protocols::AccountRepository;
use crate::{Account, AccountDraft, AccountError, edit_account};

/// Write an [`AccountDraft`] onto Account `id`, leaving its Unit, opening balance and opened date.
#[derive(Debug, Clone, PartialEq)]
pub struct EditAccount {
    pub id: AccountId,
    pub draft: AccountDraft,
}

impl EditAccount {
    /// Returns the Account as written, or refuses an Account that isn't there.
    pub fn run(self, accounts: &mut dyn AccountRepository) -> Result<Account, AccountError> {
        let mut rows = accounts.accounts();
        let outcome = edit_account(&mut rows, self.id, &self.draft);
        match &outcome {
            Ok(account) => {
                accounts.upsert_account(account.clone());
                tracing::info!(account_id = %account.id, "Edited an Account");
            }
            Err(refusal) => {
                tracing::info!(account_id = %self.id, %refusal, "Refused to edit an Account");
            }
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use lib_core::{AccountType, InstitutionId};

    use super::*;
    use crate::fake::InMemoryAccounts;

    fn draft(name: &str) -> AccountDraft {
        AccountDraft {
            name: name.into(),
            institution: InstitutionId::SEED_WESTPAC,
            account_type: AccountType::CreditCard,
            account_number: Some("42".into()),
        }
    }

    #[test]
    fn editing_an_account_writes_the_draft_and_keeps_the_fixed_fields() {
        let mut accounts = InMemoryAccounts::stub();
        let before = accounts.account(AccountId::seed(2)).expect("seeded");

        let edited = EditAccount {
            id: before.id,
            draft: draft("Everyday"),
        }
        .run(&mut accounts)
        .expect("account 2 exists");

        assert_eq!(accounts.account(before.id), Some(edited.clone()));
        assert_eq!(edited.name, "Everyday");
        assert_eq!(
            (&edited.unit, &edited.balance, edited.opened_at),
            (&before.unit, &before.balance, before.opened_at)
        );
    }

    #[test]
    fn a_missing_account_is_refused_and_nothing_is_written() {
        let mut accounts = InMemoryAccounts::stub();
        let before = accounts.clone();

        assert_eq!(
            EditAccount {
                id: AccountId::seed(999),
                draft: draft("x"),
            }
            .run(&mut accounts),
            Err(AccountError::NotFound)
        );
        assert_eq!(accounts, before);
    }
}
