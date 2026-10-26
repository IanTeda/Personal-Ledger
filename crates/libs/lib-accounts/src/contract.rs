//! The contract every stored Accounts port set keeps (ADR-0046), run against the in-memory ports
//! here and the `lib-database` adapter (#853), so the fakes the tests use can be trusted.
//!
//! Each check is a function generic over a [`PortsFactory`]: a consumer runs it from its own test,
//! since an adapter's tests need their own set-up (a fresh database per test). Each writes through
//! the Accounts Interactors, as a Client does, reloads as a restart would, and expects the same
//! rows back, in the same order. Delete goes straight to the port: it is `lib-account-deletion`'s
//! write (#854), not an Accounts Interactor.

use chrono::NaiveDate;
use lib_core::{AccountId, AccountType, InstitutionId, UnitId};

use crate::fake::InMemoryPorts;
use crate::interactor::{AddAccount, EditAccount};
use crate::{
    Account, AccountDraft, AccountError, AccountRepository, AccountUsage, AccountUse, NewAccount,
    default_accounts,
};

/// A stored set of the Accounts ports.
pub trait StoredPorts {
    fn accounts(&mut self) -> &mut dyn AccountRepository;

    fn usage(&self) -> &dyn AccountUsage;
}

/// Builds the ports a contract check runs on.
pub trait PortsFactory {
    type Ports: StoredPorts;

    /// Ports holding `seed`'s Accounts, in order, as if stored and loaded, and nothing naming any
    /// of them.
    fn create(&mut self, seed: Vec<Account>) -> Self::Ports;

    /// Ends the writes made on `ports`, then loads a fresh set from wherever they were stored, as
    /// a restart would.
    fn reload(&mut self, ports: Self::Ports) -> Self::Ports;

    /// Ends one write made on `ports`, sending its changes to storage as an adapter's owner does
    /// after each input.
    fn commit(&mut self, ports: &mut Self::Ports);

    /// Makes `uses.transactions` more Transactions and `uses.budgets` more Budgets name Account
    /// `id`, as those domains' stored rows would, so [`AccountUsage`] has something to count.
    fn name_account(&mut self, ports: &mut Self::Ports, id: AccountId, uses: AccountUse);

    /// Makes every write from now on fail, as a store that refuses it would.
    fn break_writes(&mut self, ports: &mut Self::Ports);

    /// Settles the writes sent so far, then returns `ports` reloaded to the last committed rows,
    /// with the number of failures reported naming the Accounts domain.
    fn recover(&mut self, ports: Self::Ports) -> (Self::Ports, usize);
}

impl StoredPorts for InMemoryPorts {
    fn accounts(&mut self) -> &mut dyn AccountRepository {
        &mut self.accounts
    }

    fn usage(&self) -> &dyn AccountUsage {
        &self.usage
    }
}

/// The in-memory ports are their own storage, so a reload hands them back as they are. It keeps
/// a copy as of the last commit, which a failed write falls back to.
#[derive(Debug, Default)]
pub struct InMemoryFactory {
    committed: Option<InMemoryPorts>,
    broken: bool,
    failures: usize,
}

impl PortsFactory for InMemoryFactory {
    type Ports = InMemoryPorts;

    fn create(&mut self, seed: Vec<Account>) -> InMemoryPorts {
        let mut ports = InMemoryPorts::default();
        ports.accounts.accounts = seed;
        self.committed = Some(ports.clone());
        ports
    }

    fn reload(&mut self, mut ports: InMemoryPorts) -> InMemoryPorts {
        self.commit(&mut ports);
        ports
    }

    fn commit(&mut self, ports: &mut InMemoryPorts) {
        if self.broken {
            self.failures += 1;
        } else {
            self.committed = Some(ports.clone());
        }
    }

    /// Other domains' rows aren't the Accounts writes, so the count lands in the committed copy
    /// too.
    fn name_account(&mut self, ports: &mut InMemoryPorts, id: AccountId, uses: AccountUse) {
        let add = |ports: &mut InMemoryPorts| {
            let entry = ports.usage.uses.entry(id).or_default();
            entry.transactions += uses.transactions;
            entry.budgets += uses.budgets;
        };
        add(ports);
        if let Some(committed) = self.committed.as_mut() {
            add(committed);
        }
    }

    fn break_writes(&mut self, _ports: &mut InMemoryPorts) {
        self.broken = true;
    }

    fn recover(&mut self, _ports: InMemoryPorts) -> (InMemoryPorts, usize) {
        let committed = self.committed.clone().expect("create ran first");
        (committed, std::mem::take(&mut self.failures))
    }
}

fn draft(name: &str, account_number: Option<&str>) -> AccountDraft {
    AccountDraft {
        name: name.into(),
        institution: InstitutionId::SEED_ANZ,
        account_type: AccountType::Bank,
        account_number: account_number.map(Into::into),
    }
}

fn new_account(name: &str) -> NewAccount {
    NewAccount {
        draft: draft(name, Some("1234 5678")),
        unit: UnitId::SEED_AUD,
        balance: "1520.75".parse().expect("a decimal"),
        opened_at: NaiveDate::from_ymd_opt(2026, 3, 1).expect("a date"),
    }
}

fn ids(ports: &mut impl StoredPorts) -> Vec<AccountId> {
    ports
        .accounts()
        .accounts()
        .into_iter()
        .map(|account| account.id)
        .collect()
}

/// An added Account reloads with its id, Institution, Account Type, Unit, opening balance,
/// opened date and account number.
pub fn an_added_account_reloads_the_same<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(default_accounts());

    let added = AddAccount(new_account("Holiday Fund")).run(ports.accounts());
    factory.commit(&mut ports);

    let before = ports.accounts().accounts();
    let mut ports = factory.reload(ports);
    assert_eq!(ports.accounts().accounts(), before);
    assert_eq!(
        ports.accounts().account(added.id),
        Some(Account {
            id: added.id,
            name: "Holiday Fund".into(),
            institution: InstitutionId::SEED_ANZ,
            account_type: AccountType::Bank,
            unit: UnitId::SEED_AUD,
            balance: "1520.75".parse().expect("a decimal"),
            account_number: Some("1234 5678".into()),
            opened_at: NaiveDate::from_ymd_opt(2026, 3, 1).expect("a date"),
        })
    );
}

/// An edit reloads with the draft written and the Unit, opening balance and opened date as they
/// were; clearing the account number reloads it cleared. Editing an Account that isn't there is
/// refused and writes nothing.
pub fn an_edited_account_reloads_the_same<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(default_accounts());
    let before = ports
        .accounts()
        .account(AccountId::seed(2))
        .expect("ANZ Everyday is seeded");

    let edited = EditAccount {
        id: before.id,
        draft: AccountDraft {
            name: "Everyday".into(),
            institution: InstitutionId::SEED_WESTPAC,
            account_type: AccountType::CreditCard,
            account_number: None,
        },
    }
    .run(ports.accounts())
    .expect("the Account is there");
    assert_eq!(
        EditAccount {
            id: AccountId::seed(999),
            draft: draft("Nowhere", None),
        }
        .run(ports.accounts()),
        Err(AccountError::NotFound)
    );
    factory.commit(&mut ports);

    let mut ports = factory.reload(ports);
    assert_eq!(ports.accounts().account(before.id), Some(edited.clone()));
    assert_eq!(edited.account_number, None);
    assert_eq!(
        (&edited.unit, &edited.balance, edited.opened_at),
        (&before.unit, &before.balance, before.opened_at)
    );
    assert_eq!(ports.accounts().accounts().len(), default_accounts().len());
}

/// A deleted Account is gone after a reload, and the others keep their order.
pub fn a_deleted_account_stays_deleted<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(default_accounts());

    ports.accounts().delete_account(AccountId::seed(3));
    factory.commit(&mut ports);

    let expected: Vec<_> = default_accounts()
        .into_iter()
        .map(|account| account.id)
        .filter(|id| *id != AccountId::seed(3))
        .collect();
    let mut ports = factory.reload(ports);
    assert_eq!(ids(&mut ports), expected);
}

/// Accounts list in the order they were added, the seed's first, before and after a reload.
pub fn accounts_list_in_the_order_added<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(default_accounts());

    let zeta = AddAccount(new_account("Zeta Savings")).run(ports.accounts());
    factory.commit(&mut ports);
    let alpha = AddAccount(new_account("Alpha Card")).run(ports.accounts());
    factory.commit(&mut ports);

    let mut expected: Vec<_> = default_accounts()
        .into_iter()
        .map(|account| account.id)
        .collect();
    expected.extend([zeta.id, alpha.id]);
    assert_eq!(ids(&mut ports), expected);
    let mut ports = factory.reload(ports);
    assert_eq!(ids(&mut ports), expected, "a reload keeps the order");
}

/// [`AccountUsage`] counts the Transactions and Budgets naming each Account, before and after a
/// reload; an Account nothing names, or one that isn't there, has none. Nothing is written to the
/// Account for it.
pub fn usage_counts_the_rows_naming_an_account<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(default_accounts());
    let everyday = AccountId::seed(2);
    let amex = AccountId::seed(4);
    let stored = ports.accounts().accounts();

    factory.name_account(
        &mut ports,
        everyday,
        AccountUse {
            transactions: 3,
            budgets: 1,
        },
    );
    factory.name_account(
        &mut ports,
        amex,
        AccountUse {
            transactions: 2,
            budgets: 0,
        },
    );
    factory.name_account(
        &mut ports,
        everyday,
        AccountUse {
            transactions: 1,
            budgets: 1,
        },
    );

    let check = |ports: &F::Ports| {
        assert_eq!(
            ports.usage().account_usage(everyday),
            AccountUse {
                transactions: 4,
                budgets: 2,
            }
        );
        assert_eq!(
            ports.usage().account_usage(amex),
            AccountUse {
                transactions: 2,
                budgets: 0,
            }
        );
        assert_eq!(
            ports.usage().account_usage(AccountId::seed(1)),
            AccountUse::default()
        );
        assert_eq!(
            ports.usage().account_usage(AccountId::seed(999)),
            AccountUse::default()
        );
    };
    check(&ports);
    assert_eq!(
        ports.accounts().accounts(),
        stored,
        "counting writes nothing"
    );
    let ports = factory.reload(ports);
    check(&ports);
}

/// A write that fails is reported once, naming the Accounts domain, and the ports go back to the
/// last committed rows rather than keeping the change the store refused.
pub fn a_failed_write_reloads_and_reports<F: PortsFactory>(factory: &mut F) {
    let mut ports = factory.create(default_accounts());
    let stored = ports.accounts().accounts();

    factory.break_writes(&mut ports);
    AddAccount(new_account("Holiday Fund")).run(ports.accounts());
    factory.commit(&mut ports);

    let (mut ports, failures) = factory.recover(ports);
    assert_eq!(failures, 1, "one failure naming the Accounts domain");
    assert_eq!(
        ports.accounts().accounts(),
        stored,
        "the refused write is gone"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_an_added_account_reloads_the_same() {
        an_added_account_reloads_the_same(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_an_edited_account_reloads_the_same() {
        an_edited_account_reloads_the_same(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_a_deleted_account_stays_deleted() {
        a_deleted_account_stays_deleted(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_accounts_list_in_the_order_added() {
        accounts_list_in_the_order_added(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_usage_counts_the_rows_naming_an_account() {
        usage_counts_the_rows_naming_an_account(&mut InMemoryFactory::default());
    }

    #[test]
    fn in_memory_a_failed_write_reloads_and_reports() {
        a_failed_write_reloads_and_reports(&mut InMemoryFactory::default());
    }
}
