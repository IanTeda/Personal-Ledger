//! How many Transactions the Ledger's read copy books to an Account (#852), for a Client's
//! in-memory Accounts store to recount its usage from, since it has no file to count in; the
//! adapter counts in the file (#853). The Budgets half of an Account's usage is the Budgets
//! store's (#952), so the Ledger no longer answers `AccountUsage`.

use lib_core::AccountId;

use crate::Ledger;

impl Ledger {
    /// How many Transactions are booked to Account `id`.
    pub fn account_transactions(&self, id: AccountId) -> usize {
        self.transactions
            .iter()
            .filter(|transaction| transaction.account_id == id)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use lib_accounts::{AccountDraft, NewAccount};
    use lib_core::{AccountType, InstitutionId, UnitId};

    use super::*;

    #[test]
    fn the_stubs_accounts_count_their_transactions() {
        let ledger = Ledger::stub();
        for account in ledger.accounts() {
            let transactions = ledger
                .transactions()
                .iter()
                .filter(|t| t.account_id == account.id)
                .count();
            assert_eq!(
                ledger.account_transactions(account.id),
                transactions,
                "{}",
                account.name
            );
        }
        assert!(ledger.account_transactions(AccountId::seed(2)) > 0);
        assert_eq!(ledger.account_transactions(AccountId::new()), 0);
    }

    #[test]
    fn a_new_account_has_no_transactions_until_one_is_booked_to_it() {
        let mut ledger = Ledger::stub();
        let id = lib_accounts::insert_account(
            &mut ledger.accounts,
            NewAccount {
                draft: AccountDraft {
                    name: "Fresh".into(),
                    institution: InstitutionId::SEED_ANZ,
                    account_type: AccountType::Bank,
                    account_number: None,
                },
                unit: UnitId::SEED_AUD,
                balance: "0".parse().expect("a decimal"),
                opened_at: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).expect("a date"),
            },
        )
        .id;

        assert_eq!(ledger.account_transactions(id), 0);
    }
}
