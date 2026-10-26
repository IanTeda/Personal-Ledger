//! `lib-accounts`' [`AccountUsage`] answered from the Ledger (#852), a halfway shim: the Account
//! Edit and Delete Dialogs count an Account's Transactions and Budgets from the rows themselves,
//! so an Import can't leave the figure wrong. The Transactions count moves to `lib-database` once
//! Accounts persist (#853); the Budgets count stays here until Budgets persist (#811).

use lib_accounts::{AccountUsage, AccountUse};
use lib_core::AccountId;

use crate::Ledger;

impl Ledger {
    /// How many Transactions are booked to Account `id`, and how many Budgets count it.
    pub fn account_use(&self, id: AccountId) -> AccountUse {
        AccountUse {
            transactions: self
                .transactions
                .iter()
                .filter(|transaction| transaction.account_id == id)
                .count(),
            budgets: self
                .budgets
                .all()
                .iter()
                .filter(|budget| budget.account_ids.contains(&id))
                .count(),
        }
    }
}

impl AccountUsage for Ledger {
    fn account_usage(&self, id: AccountId) -> AccountUse {
        self.account_use(id)
    }
}

#[cfg(test)]
mod tests {
    use lib_accounts::{AccountDraft, NewAccount};
    use lib_core::{AccountType, InstitutionId, UnitId};

    use super::*;

    #[test]
    fn the_stubs_account_usage_counts_its_transactions_and_budgets() {
        let ledger = Ledger::stub();
        for account in ledger.accounts() {
            let transactions = ledger
                .transactions()
                .iter()
                .filter(|t| t.account_id == account.id)
                .count();
            let budgets = ledger
                .budgets()
                .all()
                .iter()
                .filter(|b| b.account_ids.contains(&account.id))
                .count();
            assert_eq!(
                ledger.account_usage(account.id),
                AccountUse {
                    transactions,
                    budgets
                },
                "{}",
                account.name
            );
        }
        let everyday = ledger.account_usage(AccountId::seed(2));
        assert!(everyday.transactions > 0 && everyday.budgets > 0);
        assert_eq!(
            ledger.account_usage(AccountId::new()),
            AccountUse::default()
        );
    }

    #[test]
    fn a_new_account_has_no_usage_until_something_names_it() {
        let mut ledger = Ledger::stub();
        let id = ledger.add_account(NewAccount {
            draft: AccountDraft {
                name: "Fresh".into(),
                institution: InstitutionId::SEED_ANZ,
                account_type: AccountType::Bank,
                account_number: None,
            },
            unit: UnitId::SEED_AUD,
            balance: "0".parse().expect("a decimal"),
            opened_at: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).expect("a date"),
        });

        assert_eq!(ledger.account_usage(id), AccountUse::default());
    }
}
