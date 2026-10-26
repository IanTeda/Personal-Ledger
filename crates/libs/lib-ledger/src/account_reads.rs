//! The Ledger's Accounts are a read copy of the Accounts port (#853): a Client refreshes it from the
//! port after each Accounts write and on rollback ([`Ledger::refresh_accounts`]), for the Views not
//! yet modules: the Accounts page, Transactions, Budgets, Documents, Import and the Dashboard. The
//! Accounts writers still run as `LedgerCommand`s, which a Client forwards to the port, until the
//! Accounts page (#855) and `lib-account-deletion` (#854) write it themselves. The copy goes when
//! the last reader has its own port.

use lib_accounts::Account;

use crate::Ledger;

impl Ledger {
    /// Replaces the read copy of the Accounts with what the Accounts port holds, in the port's
    /// order. Returns whether anything changed, so a Client announces only a real change.
    pub fn refresh_accounts(&mut self, accounts: Vec<Account>) -> bool {
        if self.accounts == accounts {
            return false;
        }
        self.accounts = accounts;
        true
    }
}

#[cfg(test)]
mod tests {
    use lib_core::AccountId;

    use super::*;

    #[test]
    fn a_refresh_replaces_the_copy_and_says_whether_it_changed() {
        let mut ledger = Ledger::stub();
        let mut accounts = ledger.accounts().to_vec();
        assert!(!ledger.refresh_accounts(accounts.clone()));

        accounts.push(Account {
            id: AccountId::new(),
            name: "Holiday Fund".into(),
            ..accounts[1].clone()
        });
        assert!(ledger.refresh_accounts(accounts.clone()));
        assert_eq!(ledger.accounts(), accounts);
    }
}
