//! In-memory ports and the stub seed, for tests and UI-first work. Each holds its rows in a plain
//! field so a test can set up or read state directly.

use std::collections::HashMap;

use lib_core::AccountId;

use crate::{Account, AccountRepository, AccountUsage, AccountUse, default_accounts};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryAccounts {
    pub accounts: Vec<Account>,
}

impl InMemoryAccounts {
    /// The stub seed: the stub Ledger's Accounts (see [`default_accounts`]). Ids match
    /// `lib-ledger`'s stub, so its Transactions and Budgets still resolve their Accounts.
    pub fn stub() -> Self {
        Self {
            accounts: default_accounts(),
        }
    }
}

impl AccountRepository for InMemoryAccounts {
    fn accounts(&self) -> Vec<Account> {
        self.accounts.clone()
    }

    fn upsert_account(&mut self, account: Account) {
        match self.accounts.iter_mut().find(|row| row.id == account.id) {
            Some(row) => *row = account,
            None => self.accounts.push(account),
        }
    }

    fn delete_account(&mut self, id: AccountId) {
        self.accounts.retain(|row| row.id != id);
    }
}

/// Usage counts a test sets per Account; an Account not listed has none.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryAccountUsage {
    pub uses: HashMap<AccountId, AccountUse>,
}

impl AccountUsage for InMemoryAccountUsage {
    fn account_usage(&self, id: AccountId) -> AccountUse {
        self.uses.get(&id).copied().unwrap_or_default()
    }
}

/// Every Accounts port in memory, together, so a test can hand a write the ones it takes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryPorts {
    pub accounts: InMemoryAccounts,
    pub usage: InMemoryAccountUsage,
}

impl InMemoryPorts {
    /// The stub Accounts with nothing naming any of them.
    pub fn stub() -> Self {
        Self {
            accounts: InMemoryAccounts::stub(),
            usage: InMemoryAccountUsage::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upserting_an_account_replaces_the_row_with_its_id_or_appends_it() {
        let mut ports = InMemoryPorts::stub();
        let seeded = ports.accounts.accounts().len();
        let mut wallet = ports.accounts.account(AccountId::seed(1)).expect("seeded");
        wallet.name = "Purse".into();
        ports.accounts.upsert_account(wallet.clone());
        let fresh = Account {
            id: AccountId::new(),
            ..wallet.clone()
        };
        ports.accounts.upsert_account(fresh.clone());

        assert_eq!(ports.accounts.accounts().len(), seeded + 1);
        assert_eq!(ports.accounts.account(AccountId::seed(1)), Some(wallet));
        assert_eq!(ports.accounts.accounts().last(), Some(&fresh));

        ports.accounts.delete_account(AccountId::seed(1));
        assert_eq!(ports.accounts.account(AccountId::seed(1)), None);
    }

    #[test]
    fn an_account_no_test_names_is_unused() {
        let ports = InMemoryPorts::stub();
        assert_eq!(
            ports.usage.account_usage(AccountId::seed(1)),
            AccountUse::default()
        );
    }
}
