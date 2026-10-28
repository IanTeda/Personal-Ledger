//! The Institutions ports as the Desktop holds them: the store `shell::router::build` is given,
//! and the step that keeps the Ledger's read copy of the Institutions in line after an Institutions
//! write. The write is committed by the input it was made in (`crate::input::send`), not here.
//!
//! The Usual unit's options are the Ledger's Units, copied when the Ledger read copy is refreshed.
//!
//! A delete reads what names an Institution from the Accounts store, which the Units and Bills
//! reads share.
//!
//! The ports are one value shared two ways: the Institutions module's Presenters read and write
//! them as [`InstitutionsPorts`], and the Ledger store reads the rows back from them to refresh its
//! copy and reloads them after a failed write.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{App, Entity};
use lib_core::InstitutionId;
use lib_database::{InstitutionsStore, SharedAccountsStore};
use lib_institutions::fake::InMemoryInstitutionUnits;
use lib_institutions::{
    InstitutionRepository, InstitutionUnit, InstitutionUnits, InstitutionUsage, InstitutionUse,
    InstitutionsPorts,
};
use lib_ledger::Ledger;

use crate::ledger_store::LedgerStore;

/// The Institutions ports the Institutions module and the Ledger store share.
pub type DesktopPorts = Rc<RefCell<LedgerBackedPorts>>;

/// `store`'s Institutions, with the Accounts naming each read from the Accounts store.
pub struct LedgerBackedPorts {
    store: Box<dyn InstitutionsStore>,
    usage: AccountsUsage,
    /// The Units a Usual unit may name, copied from the Ledger (a halfway shim until Units answer
    /// through their own port, as the Accounts' do).
    units: InMemoryInstitutionUnits,
}

/// What names an Institution, answered from the Accounts store on each ask, so an Account added or
/// moved while a Delete Dialog is open is seen when it confirms.
struct AccountsUsage(SharedAccountsStore);

impl InstitutionUsage for AccountsUsage {
    fn institution_usage(&self, id: InstitutionId) -> InstitutionUse {
        self.0.borrow().institution_usage(id)
    }
}

impl LedgerBackedPorts {
    pub fn shared(
        store: Box<dyn InstitutionsStore>,
        accounts: SharedAccountsStore,
    ) -> DesktopPorts {
        Rc::new(RefCell::new(Self {
            store,
            usage: AccountsUsage(accounts),
            units: InMemoryInstitutionUnits::default(),
        }))
    }

    /// Copies `ledger`'s Units, for the next Dialog to offer as the Usual unit's options.
    pub fn copy_units(&mut self, ledger: &Ledger) {
        self.units.units = ledger
            .units()
            .iter()
            .map(|unit| InstitutionUnit {
                id: unit.id,
                code: unit.code.clone(),
                is_active: unit.is_active,
            })
            .collect();
    }

    /// `ledger`'s own Institutions in memory, with the Accounts naming them read from `accounts`,
    /// for a test that needs no storage.
    pub fn in_memory(ledger: &Ledger, accounts: SharedAccountsStore) -> DesktopPorts {
        let ports = Self::shared(
            Box::new(lib_institutions::fake::InMemoryInstitutions {
                institutions: ledger.institutions().to_vec(),
            }),
            accounts,
        );
        ports.borrow_mut().copy_units(ledger);
        ports
    }

    /// Puts the rows back as they are stored, after a write failed (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading them back; the rows in memory are left as they were.
    pub fn reload(&mut self) -> lib_database::Result<()> {
        self.store.reload()
    }
}

impl InstitutionsPorts for LedgerBackedPorts {
    fn institutions(&self) -> &dyn InstitutionRepository {
        self.store.institutions()
    }

    fn institutions_mut(&mut self) -> &mut dyn InstitutionRepository {
        self.store.institutions_mut()
    }

    fn usage(&self) -> &dyn InstitutionUsage {
        &self.usage
    }

    fn units(&self) -> &dyn InstitutionUnits {
        &self.units
    }
}

/// The same ports as the store they wrap, for the Unit Deletion adapter, which writes it with the
/// Units store in one delete (#903).
impl InstitutionsStore for LedgerBackedPorts {
    fn institutions(&self) -> &dyn InstitutionRepository {
        self.store.institutions()
    }

    fn institutions_mut(&mut self) -> &mut dyn InstitutionRepository {
        self.store.institutions_mut()
    }

    fn reload(&mut self) -> lib_database::Result<()> {
        self.store.reload()
    }
}

/// Brings the Ledger read copy of the Institutions in line with the ports, announcing
/// `Domain::Institutions` when it changed. Run before and after every Institutions event and
/// Dialog confirm, so Accounts see a write at once.
pub fn refresh(ledger: &Entity<LedgerStore>, cx: &mut App) {
    ledger.update(cx, |store, cx| store.refresh_institutions(cx));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_ports_hold_the_ledgers_institutions() {
        let ledger = Ledger::stub();
        let accounts = crate::accounts::LedgerBackedPorts::in_memory(&ledger);
        let ports = LedgerBackedPorts::in_memory(&ledger, accounts);
        let ports = ports.borrow();

        assert_eq!(
            InstitutionsPorts::institutions(&*ports).institutions(),
            ledger.institutions()
        );
    }

    #[test]
    fn usage_is_the_accounts_naming_the_institution() {
        let ledger = Ledger::stub();
        let accounts = crate::accounts::LedgerBackedPorts::in_memory(&ledger);
        let ports = LedgerBackedPorts::in_memory(&ledger, accounts);
        let ports = ports.borrow();

        let wallet: Vec<_> = ledger
            .accounts()
            .iter()
            .filter(|account| account.institution == InstitutionId::SEED_PERSONAL)
            .map(|account| account.name.clone())
            .collect();
        assert!(!wallet.is_empty());
        assert_eq!(
            InstitutionsPorts::usage(&*ports)
                .institution_usage(InstitutionId::SEED_PERSONAL)
                .accounts,
            wallet
        );
        assert!(
            !InstitutionsPorts::usage(&*ports)
                .institution_usage(InstitutionId::SEED_SUPER_FUND)
                .is_used()
        );
    }
}
