//! The Accounts ports as the Desktop holds them: the store `shell::router::build` is given, with
//! the Institutions and Units copied from the Ledger read copy (a halfway shim until each answers
//! the Accounts' selects through its own port) and the Budgets count read from the Budgets store
//! (#952), and the step that keeps the Ledger's read copy of the Accounts in line after an
//! Accounts write. The write is committed by the input it was made in (`crate::input::send`), not
//! here.
//!
//! The ports are one value shared several ways: the Accounts module's Presenters read and write
//! them as [`AccountsPorts`], and the same value is the `AccountsStore` the Ledger's read copy,
//! the Units and Bills reads and `lib-account-deletion`'s adapter hold.
//!
//! An in-memory store's Transactions count is recounted from the Ledger before each read, since
//! it has no file to count in; the adapter counts in the file.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{App, Entity};
use lib_accounts::fake::{InMemoryAccountInstitutions, InMemoryAccountUnits};
use lib_accounts::{
    AccountInstitution, AccountInstitutions, AccountRepository, AccountUnit, AccountUnits,
    AccountUsage, AccountUse, AccountsPorts,
};
use lib_bills::BillAccount;
use lib_core::{AccountId, UnitId};
use lib_database::{AccountsStore, SharedAccountsStore, SharedBudgetsStore};
use lib_ledger::Ledger;
use lib_units::Holding;

use crate::ledger_store::LedgerStore;

/// The Accounts ports the Accounts module and the Ledger share.
pub type DesktopPorts = Rc<RefCell<LedgerBackedPorts>>;

/// `store`'s Accounts, with the Institutions and Units an Account may name copied from the
/// Ledger, and the Budgets that count each read from `budgets`.
pub struct LedgerBackedPorts {
    store: Box<dyn AccountsStore>,
    institutions: InMemoryAccountInstitutions,
    units: InMemoryAccountUnits,
    budgets: SharedBudgetsStore,
}

impl LedgerBackedPorts {
    pub fn shared(store: Box<dyn AccountsStore>, budgets: SharedBudgetsStore) -> DesktopPorts {
        Rc::new(RefCell::new(Self {
            store,
            institutions: InMemoryAccountInstitutions::default(),
            units: InMemoryAccountUnits::default(),
            budgets,
        }))
    }

    /// `ledger`'s own Accounts in memory, reading `budgets`, for a test that needs no storage.
    pub fn in_memory(ledger: &Ledger, budgets: SharedBudgetsStore) -> DesktopPorts {
        let ports = Self::shared(
            Box::new(lib_accounts::fake::InMemoryPorts {
                accounts: lib_accounts::fake::InMemoryAccounts {
                    accounts: ledger.accounts().to_vec(),
                },
                ..Default::default()
            }),
            budgets,
        );
        ports.borrow_mut().copy_from_ledger(ledger);
        ports
    }

    fn copy_from_ledger(&mut self, ledger: &Ledger) {
        self.institutions.institutions = ledger
            .institutions()
            .iter()
            .map(|institution| AccountInstitution {
                id: institution.id,
                name: institution.name.clone(),
                usual_unit: institution.usual_unit,
            })
            .collect();
        self.units.units = ledger
            .units()
            .iter()
            .map(|unit| AccountUnit {
                id: unit.id,
                code: unit.code.clone(),
                is_active: unit.is_active,
                precision: unit.precision.get(),
            })
            .collect();
        self.units.base = ledger.base_unit_row().map(|unit| unit.id);
        if let Some(usage) = self.store.usage_to_count() {
            usage.uses = transaction_uses(ledger);
        }
    }
}

impl AccountUsage for LedgerBackedPorts {
    /// The Transactions from the Accounts store, the Budgets from the Budgets store.
    fn account_usage(&self, id: AccountId) -> AccountUse {
        AccountUse {
            transactions: self.store.usage().account_usage(id).transactions,
            budgets: self.budgets.borrow().account_budgets(id),
        }
    }
}

impl AccountsPorts for LedgerBackedPorts {
    fn accounts(&self) -> &dyn AccountRepository {
        self.store.accounts()
    }

    fn accounts_mut(&mut self) -> &mut dyn AccountRepository {
        self.store.accounts_mut()
    }

    fn usage(&self) -> &dyn AccountUsage {
        self
    }

    fn institutions(&self) -> &dyn AccountInstitutions {
        &self.institutions
    }

    fn units(&self) -> &dyn AccountUnits {
        &self.units
    }
}

/// The same ports as the store they wrap, for the Ledger's read copy, the Units and Bills reads
/// and the deletion adapter.
impl AccountsStore for LedgerBackedPorts {
    fn accounts(&self) -> &dyn AccountRepository {
        self.store.accounts()
    }

    fn accounts_mut(&mut self) -> &mut dyn AccountRepository {
        self.store.accounts_mut()
    }

    fn usage(&self) -> &dyn AccountUsage {
        self.store.usage()
    }

    fn usage_to_count(&mut self) -> Option<&mut lib_accounts::fake::InMemoryAccountUsage> {
        self.store.usage_to_count()
    }

    fn holdings(&self, unit: UnitId) -> Vec<Holding> {
        self.store.holdings(unit)
    }

    fn unit_accounts(&self, unit: UnitId) -> usize {
        self.store.unit_accounts(unit)
    }

    fn bill_accounts(&self) -> Vec<BillAccount> {
        self.store.bill_accounts()
    }

    fn reload(&mut self) -> lib_database::Result<()> {
        self.store.reload()
    }
}

/// Copies the Institutions and Units from `ledger` into `ports`, and recounts an in-memory store's
/// Transactions, for the next Accounts event and a view model drawn outside one.
pub fn copy_from_ledger(ports: &DesktopPorts, ledger: &Ledger) {
    ports.borrow_mut().copy_from_ledger(ledger);
}

/// Brings the Ledger read copy of the Accounts in line with the ports, announcing
/// `Domain::Accounts` when it changed, and copies the Ledger's reads back for the next Accounts
/// event. Run before and after every Accounts event and Dialog confirm, so the other Views see a
/// write at once.
pub fn refresh(ports: &DesktopPorts, ledger: &Entity<LedgerStore>, cx: &mut App) {
    ledger.update(cx, |store, cx| store.refresh_accounts(cx));
    copy_from_ledger(ports, ledger.read(cx).ledger());
}

/// Each of `ledger`'s Accounts with the Transactions booked to it, as an in-memory store's usage
/// holds them. Its Budgets half is the Budgets store's, so it is left at none.
fn transaction_uses(ledger: &Ledger) -> std::collections::HashMap<AccountId, AccountUse> {
    ledger
        .accounts()
        .iter()
        .map(|account| {
            let uses = AccountUse {
                transactions: ledger.account_transactions(account.id),
                budgets: 0,
            };
            (account.id, uses)
        })
        .collect()
}

/// Recounts an in-memory store's Transactions from `ledger`'s; the adapter counts in the file, so
/// it is left alone.
pub fn recount(store: &RefCell<dyn AccountsStore>, ledger: &Ledger) {
    if let Some(usage) = store.borrow_mut().usage_to_count() {
        usage.uses = transaction_uses(ledger);
    }
}

/// What names Account `id`, as the Delete Dialog quotes it: the Transactions from the Accounts
/// store, the Budgets from the Budgets store (#952).
pub fn usage(
    store: &SharedAccountsStore,
    budgets: &SharedBudgetsStore,
    ledger: &Ledger,
    id: AccountId,
) -> AccountUse {
    recount(store, ledger);
    AccountUse {
        transactions: store.borrow().usage().account_usage(id).transactions,
        budgets: budgets.borrow().account_budgets(id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_transactions_are_counted_from_the_ledger_and_the_budgets_from_their_store() {
        let ledger = Ledger::stub();
        let budgets = crate::budgets::ports::in_memory(&ledger);
        let ports = LedgerBackedPorts::in_memory(&ledger, budgets.clone());
        let store: SharedAccountsStore = ports.clone();
        let everyday = AccountId::seed(2);
        let expected = AccountUse {
            transactions: ledger.account_transactions(everyday),
            budgets: 2,
        };

        assert_eq!(usage(&store, &budgets, &ledger, everyday), expected);
        assert!(expected.transactions > 0);
        assert_eq!(
            AccountsPorts::usage(&*ports.borrow()).account_usage(everyday),
            expected
        );

        if let Some(memory) = budgets.borrow_mut().memory_mut() {
            memory.budgets.truncate(1);
        }
        assert_eq!(
            usage(&store, &budgets, &ledger, everyday).budgets,
            1,
            "read from the store on each ask"
        );
    }

    #[test]
    fn the_ports_offer_the_ledgers_institutions_units_and_base_unit() {
        let ledger = Ledger::stub();
        let ports =
            LedgerBackedPorts::in_memory(&ledger, crate::budgets::ports::in_memory(&ledger));
        let ports = ports.borrow();

        assert_eq!(
            AccountsPorts::institutions(&*ports)
                .account_institutions()
                .len(),
            ledger.institutions().len()
        );
        let units = AccountsPorts::units(&*ports);
        assert_eq!(units.account_units().len(), ledger.units().len());
        assert_eq!(units.base_unit(), Some(UnitId::SEED_AUD));
    }
}
