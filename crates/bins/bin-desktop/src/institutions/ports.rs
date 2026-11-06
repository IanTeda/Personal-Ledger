//! The Institutions ports as the Desktop holds them: the Institutions store `shell::router::build`
//! shares, and the announcement that tells the Views reading the Institutions of a write. The
//! write is committed by the input it was made in (`crate::input::send`), not here. The Ledger
//! keeps no read copy of the Institutions (#1053): the Accounts page reads them from the store
//! through `lib-database`'s `StoredAccountReads`.
//!
//! The ports hold the store shared rather than owned: the Accounts' reads hold it for their
//! Institution select while these ports' reads hold the Accounts ports, so a store owned here
//! and read there would be a cycle of `Rc`s that outlives the Ledger it was built on.
//!
//! The Usual unit's options are read from the Units store, through `lib-database`'s
//! [`StoredInstitutionReads`] (#1052).
//!
//! A delete reads what names an Institution from the Accounts store, which the Units and Bills
//! reads share.
//!
//! The ports are one value shared two ways: the Institutions module's Presenters read and write
//! them as [`InstitutionsPorts`], and the Ledger store reloads them after a failed write.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gpui::{App, Entity};
use lib_core::InstitutionId;
use lib_database::{
    InstitutionsStore, SharedAccountsStore, SharedInstitutionsStore, SharedUnitsStore,
    StoredInstitutionReads,
};
use lib_institutions::{
    Institution, InstitutionRepository, InstitutionUnits, InstitutionUsage, InstitutionsPorts,
};

use crate::ledger_store::LedgerStore;
use crate::write_count::WriteCount;

/// The Institutions ports the Institutions module and the Ledger store share.
pub type DesktopPorts = Rc<RefCell<LedgerBackedPorts>>;

/// The shared Institutions store's Institutions, borrowed for each call. A write counts, so the
/// ports can tell whether an input wrote.
struct SharedInstitutions {
    store: SharedInstitutionsStore,
    writes: WriteCount,
}

impl InstitutionRepository for SharedInstitutions {
    fn institutions(&self) -> Vec<Institution> {
        self.store.borrow().institutions().institutions()
    }

    fn upsert_institution(&mut self, institution: Institution) {
        self.writes.bump();
        self.store
            .borrow_mut()
            .institutions_mut()
            .upsert_institution(institution);
    }

    fn delete_institution(&mut self, id: InstitutionId) {
        self.writes.bump();
        self.store
            .borrow_mut()
            .institutions_mut()
            .delete_institution(id);
    }
}

/// The shared store's Institutions, with the Accounts naming each and the Units a Usual unit may
/// name read from the Accounts and Units stores.
pub struct LedgerBackedPorts {
    store: SharedInstitutionsStore,
    institutions: SharedInstitutions,
    reads: StoredInstitutionReads,
    /// The write count the last announcement saw, so a write is announced once.
    announced: Cell<u64>,
}

impl LedgerBackedPorts {
    pub fn shared(
        store: SharedInstitutionsStore,
        accounts: SharedAccountsStore,
        units: SharedUnitsStore,
    ) -> DesktopPorts {
        Rc::new(RefCell::new(Self {
            institutions: SharedInstitutions {
                store: store.clone(),
                writes: WriteCount::default(),
            },
            store,
            reads: StoredInstitutionReads { accounts, units },
            announced: Cell::new(0),
        }))
    }

    /// Whether these ports were written since the last ask.
    fn take_written(&self) -> bool {
        let writes = self.institutions.writes.get();
        writes != self.announced.replace(writes)
    }
}

impl InstitutionsPorts for LedgerBackedPorts {
    fn institutions(&self) -> &dyn InstitutionRepository {
        &self.institutions
    }

    fn institutions_mut(&mut self) -> &mut dyn InstitutionRepository {
        &mut self.institutions
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
        &self.institutions
    }

    fn institutions_mut(&mut self) -> &mut dyn InstitutionRepository {
        &mut self.institutions
    }

    fn reload(&mut self) -> lib_database::Result<()> {
        self.store.borrow_mut().reload()
    }
}

/// The stub Institutions as an in-memory store, for a test that needs no storage.
pub fn in_memory_store() -> SharedInstitutionsStore {
    Rc::new(RefCell::new(
        lib_institutions::fake::InMemoryInstitutions::stub(),
    ))
}

/// Puts the Institutions back as stored, after a write that named them failed, and announces
/// `Domain::Institutions` so every reader redraws (ADR-0046).
///
/// # Errors
/// Returns the error reading them back; the ports are left as they were.
pub fn reload(ledger: &Entity<LedgerStore>, cx: &mut App) -> lib_database::Result<()> {
    let ports = ledger.read(cx).institutions_ports().clone();
    InstitutionsStore::reload(&mut *ports.borrow_mut())?;
    ledger.update(cx, |store, cx| store.announce_institutions(cx));
    Ok(())
}

/// Announces `Domain::Institutions` when the ports were written since the last announcement, so
/// the Accounts page sees a write at once. Run before and after every Institutions event and
/// Dialog confirm.
pub fn refresh(ports: &DesktopPorts, ledger: &Entity<LedgerStore>, cx: &mut App) {
    if ports.borrow().take_written() {
        ledger.update(cx, |store, cx| store.announce_institutions(cx));
    }
}

#[cfg(test)]
mod tests {
    use lib_core::InstitutionId;
    use lib_database::FakeStores;

    use super::*;

    fn stub() -> FakeStores {
        FakeStores::stub(chrono::Local::now().date_naive())
    }

    fn ports(stub: &FakeStores) -> DesktopPorts {
        let accounts = crate::accounts::LedgerBackedPorts::in_memory(
            crate::accounts::ports::in_memory_reads(stub),
        );
        LedgerBackedPorts::shared(
            in_memory_store(),
            accounts,
            crate::bills::ports::in_memory_units(chrono::Utc::now()),
        )
    }

    #[test]
    fn in_memory_ports_hold_the_stub_institutions() {
        let ports = ports(&stub());
        let ports = ports.borrow();

        assert_eq!(
            InstitutionsPorts::institutions(&*ports).institutions(),
            lib_institutions::default_institutions()
        );
    }

    #[test]
    fn a_write_through_the_ports_lands_in_the_shared_store_and_is_noted_once() {
        let ports = ports(&stub());
        let mut held = ports.borrow_mut();
        assert!(!held.take_written());

        let mut personal = InstitutionsPorts::institutions(&*held)
            .institution(InstitutionId::SEED_PERSONAL)
            .expect("the stub's Personal Institution");
        personal.name = "Renamed".into();
        InstitutionsPorts::institutions_mut(&mut *held).upsert_institution(personal);

        assert_eq!(
            held.store
                .borrow()
                .institutions()
                .institution(InstitutionId::SEED_PERSONAL)
                .map(|row| row.name),
            Some("Renamed".to_owned())
        );
        assert!(held.take_written());
        assert!(!held.take_written());
    }

    #[test]
    fn usage_is_the_accounts_naming_the_institution() {
        let stub = stub();
        let ports = ports(&stub);
        let ports = ports.borrow();

        let wallet: Vec<_> = stub
            .accounts
            .accounts
            .accounts
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
