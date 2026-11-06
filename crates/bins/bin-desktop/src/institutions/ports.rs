//! The Institutions ports as the Desktop holds them: the store `shell::router::build` is given,
//! and the step that keeps the Ledger's read copy of the Institutions in line after an Institutions
//! write. The write is committed by the input it was made in (`crate::input::send`), not here.
//!
//! The Usual unit's options are read from the Units store, through `lib-database`'s
//! [`StoredInstitutionReads`] (#1052).
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
use lib_database::{
    InstitutionsStore, SharedAccountsStore, SharedUnitsStore, StoredInstitutionReads,
};
use lib_institutions::{
    InstitutionRepository, InstitutionUnits, InstitutionUsage, InstitutionsPorts,
};
use lib_ledger::Ledger;

use crate::ledger_store::LedgerStore;

/// The Institutions ports the Institutions module and the Ledger store share.
pub type DesktopPorts = Rc<RefCell<LedgerBackedPorts>>;

/// `store`'s Institutions, with the Accounts naming each and the Units a Usual unit may name read
/// from the Accounts and Units stores.
pub struct LedgerBackedPorts {
    store: Box<dyn InstitutionsStore>,
    reads: StoredInstitutionReads,
}

impl LedgerBackedPorts {
    pub fn shared(
        store: Box<dyn InstitutionsStore>,
        accounts: SharedAccountsStore,
        units: SharedUnitsStore,
    ) -> DesktopPorts {
        Rc::new(RefCell::new(Self {
            store,
            reads: StoredInstitutionReads { accounts, units },
        }))
    }

    /// `ledger`'s own Institutions in memory, with the Accounts naming them and the Units read
    /// from `accounts` and `units`, for a test that needs no storage.
    pub fn in_memory(
        ledger: &Ledger,
        accounts: SharedAccountsStore,
        units: SharedUnitsStore,
    ) -> DesktopPorts {
        Self::shared(
            Box::new(lib_institutions::fake::InMemoryInstitutions {
                institutions: ledger.institutions().to_vec(),
            }),
            accounts,
            units,
        )
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
        &self.reads
    }

    fn units(&self) -> &dyn InstitutionUnits {
        &self.reads
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
    use lib_core::InstitutionId;

    use super::*;

    #[test]
    fn in_memory_ports_hold_the_ledgers_institutions() {
        let ledger = Ledger::stub();
        let accounts = crate::accounts::LedgerBackedPorts::in_memory(
            &ledger,
            crate::accounts::ports::in_memory_reads(&ledger),
        );
        let ports = LedgerBackedPorts::in_memory(
            &ledger,
            accounts,
            crate::bills::ports::in_memory_units(chrono::Utc::now()),
        );
        let ports = ports.borrow();

        assert_eq!(
            InstitutionsPorts::institutions(&*ports).institutions(),
            ledger.institutions()
        );
    }

    #[test]
    fn usage_is_the_accounts_naming_the_institution() {
        let ledger = Ledger::stub();
        let accounts = crate::accounts::LedgerBackedPorts::in_memory(
            &ledger,
            crate::accounts::ports::in_memory_reads(&ledger),
        );
        let ports = LedgerBackedPorts::in_memory(
            &ledger,
            accounts,
            crate::bills::ports::in_memory_units(chrono::Utc::now()),
        );
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
