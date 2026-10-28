//! In-memory ports and the stub seed, for tests and UI-first work. Each holds its rows in a plain
//! field so a test can set up or read state directly.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use lib_core::{AccountId, InstitutionId, UnitId};
use lib_toast::{Notice, Notify};

use crate::protocols::{
    AccountInstitution, AccountInstitutions, AccountUnit, AccountUnits, AccountsDialog,
    AccountsOutcome, AccountsPorts, AccountsRouter, SharedPorts,
};
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

/// The Institutions an Account may be at, as a test sets them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryAccountInstitutions {
    pub institutions: Vec<AccountInstitution>,
}

impl AccountInstitutions for InMemoryAccountInstitutions {
    fn account_institutions(&self) -> Vec<AccountInstitution> {
        self.institutions.clone()
    }
}

/// The Units an Account may count in, and the base Unit, as a test sets them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryAccountUnits {
    pub units: Vec<AccountUnit>,
    pub base: Option<UnitId>,
}

impl AccountUnits for InMemoryAccountUnits {
    fn account_units(&self) -> Vec<AccountUnit> {
        self.units.clone()
    }

    fn base_unit(&self) -> Option<UnitId> {
        self.base
    }
}

/// The stub Institutions the stub Accounts name, with their seed ids, in the Institutions' order:
/// the Personal Institution, ANZ, Amex, Vanguard, Westpac and the crypto exchange.
pub fn stub_institutions() -> Vec<AccountInstitution> {
    [
        (InstitutionId::SEED_PERSONAL, "Personal"),
        (InstitutionId::SEED_ANZ, "ANZ Banking Group"),
        (InstitutionId::SEED_AMEX, "American Express"),
        (InstitutionId::SEED_VANGUARD, "Vanguard Investments"),
        (InstitutionId::SEED_WESTPAC, "Westpac Banking"),
        (
            InstitutionId::SEED_CRYPTO_EXCHANGE,
            "Cryptocurrency Exchange",
        ),
    ]
    .into_iter()
    .map(|(id, name)| AccountInstitution {
        id,
        name: name.to_string(),
        usual_unit: None,
    })
    .collect()
}

/// The stub Units the stub Accounts count in: aud (the base), the vas fund and btc.
pub fn stub_units() -> Vec<AccountUnit> {
    [
        (UnitId::SEED_AUD, "aud", true),
        (UnitId::SEED_VAS, "vas", false),
        (UnitId::SEED_BTC, "btc", false),
    ]
    .into_iter()
    .map(|(id, code, is_currency)| AccountUnit {
        id,
        code: code.to_string(),
        is_active: true,
        is_currency,
    })
    .collect()
}

/// Every Accounts port in memory, together, so a test can hand a write the ones it takes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct InMemoryPorts {
    pub accounts: InMemoryAccounts,
    pub usage: InMemoryAccountUsage,
    pub institutions: InMemoryAccountInstitutions,
    pub units: InMemoryAccountUnits,
}

impl InMemoryPorts {
    /// The stub Accounts with nothing naming any of them, the stub Institutions and Units they
    /// name, and aud as the base Unit.
    pub fn stub() -> Self {
        Self {
            accounts: InMemoryAccounts::stub(),
            usage: InMemoryAccountUsage::default(),
            institutions: InMemoryAccountInstitutions {
                institutions: stub_institutions(),
            },
            units: InMemoryAccountUnits {
                units: stub_units(),
                base: Some(UnitId::SEED_AUD),
            },
        }
    }

    /// These ports shared, as the Presenters hold them, with a handle a test reads them back
    /// through.
    pub fn shared(self) -> (Rc<RefCell<Self>>, SharedPorts) {
        let ports = Rc::new(RefCell::new(self));
        let shared: SharedPorts = ports.clone();
        (ports, shared)
    }
}

impl AccountsPorts for InMemoryPorts {
    fn accounts(&self) -> &dyn AccountRepository {
        &self.accounts
    }

    fn accounts_mut(&mut self) -> &mut dyn AccountRepository {
        &mut self.accounts
    }

    fn usage(&self) -> &dyn AccountUsage {
        &self.usage
    }

    fn institutions(&self) -> &dyn AccountInstitutions {
        &self.institutions
    }

    fn units(&self) -> &dyn AccountUnits {
        &self.units
    }
}

/// What a Presenter asked its Router for, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouterCall {
    OpenDialog(AccountsDialog),
    OpenDelete(AccountId),
    OpenLedger(AccountId),
    CloseDialog(Option<AccountsOutcome>),
    Notice(Notice),
}

/// A Router that does nothing but record each call, in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecordingRouter {
    pub calls: Vec<RouterCall>,
}

impl RecordingRouter {
    /// The calls made since the last `take`.
    pub fn take(&mut self) -> Vec<RouterCall> {
        std::mem::take(&mut self.calls)
    }
}

impl AccountsRouter for RecordingRouter {
    /// Always opens: the recording Router builds no Presenter that could fail.
    fn open_dialog(&mut self, dialog: AccountsDialog) -> bool {
        self.calls.push(RouterCall::OpenDialog(dialog));
        true
    }

    fn open_delete(&mut self, id: AccountId) -> bool {
        self.calls.push(RouterCall::OpenDelete(id));
        true
    }

    fn open_ledger(&mut self, id: AccountId) {
        self.calls.push(RouterCall::OpenLedger(id));
    }

    fn close_dialog(&mut self, outcome: Option<AccountsOutcome>) {
        self.calls.push(RouterCall::CloseDialog(outcome));
    }
}

impl Notify for RecordingRouter {
    fn notify(&mut self, notice: Notice) {
        self.calls.push(RouterCall::Notice(notice));
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
