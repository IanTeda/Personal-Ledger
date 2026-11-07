//! `lib-accounts`' ports answered over the stores: the Accounts from the Accounts store, and what
//! names an Account and the Units and Institutions it may name from the Transactions, Budgets,
//! Units and Institutions stores through [`StoredAccountReads`] (#993, #1052, #1053, #1084).

use lib_accounts::{
    Account, AccountInstitutions, AccountRepository, AccountUnits, AccountUsage, AccountsPorts,
};
use lib_core::AccountId;

use crate::adapter::store::{SharedAccountsStore, StoredAccountReads};

/// The shared Accounts store's Accounts, borrowed for each call: the store's guard can't be lent
/// out of its `RefCell`.
struct Accounts(SharedAccountsStore);

impl AccountRepository for Accounts {
    fn accounts(&self) -> Vec<Account> {
        self.0.borrow().accounts().accounts()
    }

    fn upsert_account(&mut self, account: Account) {
        self.0.borrow_mut().accounts_mut().upsert_account(account);
    }

    fn delete_account(&mut self, id: AccountId) {
        self.0.borrow_mut().accounts_mut().delete_account(id);
    }
}

/// [`lib_accounts::AccountsPorts`] over the shared Accounts store, and the reads over the stores of
/// the domains an Account names or is named by. A write reaches the store's recorder, so the commit
/// names the Accounts in what the input wrote.
pub struct StoredAccountsPorts {
    accounts: Accounts,
    reads: StoredAccountReads,
}

impl StoredAccountsPorts {
    pub fn new(accounts: SharedAccountsStore, reads: StoredAccountReads) -> Self {
        Self {
            accounts: Accounts(accounts),
            reads,
        }
    }
}

impl AccountsPorts for StoredAccountsPorts {
    fn accounts(&self) -> &dyn AccountRepository {
        &self.accounts
    }

    fn accounts_mut(&mut self) -> &mut dyn AccountRepository {
        &mut self.accounts
    }

    fn usage(&self) -> &dyn AccountUsage {
        &self.reads
    }

    fn institutions(&self) -> &dyn AccountInstitutions {
        &self.reads
    }

    fn units(&self) -> &dyn AccountUnits {
        &self.reads
    }
}

#[cfg(test)]
mod tests {
    use lib_account_deletion::interactor::DeleteAccount;

    use crate::ports::test_support::{fakes, file};
    use crate::{Domain, LedgerPorts};

    #[test]
    fn an_accounts_write_reports_the_accounts_written_on_either() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        for ledger in [fakes(), file(dir.path())] {
            let LedgerPorts {
                accounts, commit, ..
            } = ledger;
            let mut account = accounts
                .borrow()
                .accounts()
                .accounts()
                .into_iter()
                .next()
                .expect("a sample Account");
            account.name.push_str(" renamed");

            accounts.borrow_mut().accounts_mut().upsert_account(account);

            assert_eq!(commit.commit().wrote, [Domain::Accounts]);
        }
    }

    #[test]
    fn an_account_delete_reports_the_accounts_and_its_transactions_on_either() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        for ledger in [fakes(), file(dir.path())] {
            let LedgerPorts {
                accounts,
                account_deletion,
                commit,
                ..
            } = ledger;
            let wallet = accounts
                .borrow()
                .accounts()
                .accounts()
                .into_iter()
                .find(|account| account.name == "Wallet")
                .expect("a sample Account")
                .id;

            DeleteAccount { id: wallet }
                .run(&mut *account_deletion.borrow_mut())
                .expect("nothing pays from the Wallet");

            let wrote = commit.commit().wrote;
            assert!(wrote.contains(&Domain::Accounts), "{wrote:?}");
            assert!(wrote.contains(&Domain::Transactions), "{wrote:?}");
            assert!(accounts.borrow().accounts().account(wallet).is_none());
        }
    }
}
