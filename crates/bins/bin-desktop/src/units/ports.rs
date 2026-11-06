//! The Units ports as the Desktop holds them: the Units store `shell::router::build` shares, with
//! `UnitUsage` and `UnitHoldings` read from the Accounts, Transactions, Bills, Budgets, Inventory
//! and Institutions stores through `lib-database`'s [`StoredUnitReads`] (#993), and the
//! announcement that tells the Views reading the Units of a write. The write is committed by the
//! input it was made in (`crate::input::send`), not here. The Ledger keeps no read copy of the
//! Units (#1052).
//!
//! Usage and holdings are counted in the stores on each ask, whether they are the adapters or
//! `Stores::fake`'s in-memory ones, so nothing is copied from the Ledger.
//!
//! The ports hold the store shared rather than owned: the Accounts and Institutions read the same
//! store for their Unit choices, and the reads here hold their ports, so a store owned here and
//! read there would be a cycle of `Rc`s that outlives the Ledger it was built on.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gpui::{App, Entity};
use lib_core::{PriceSourceId, UnitId};
use lib_database::{SharedUnitsStore, StoredUnitReads};
use lib_units::{
    BaseUnitRepository, PortsMut, PriceSourceRecord, PriceSourceRepository, UnitHoldings,
    UnitRepository, UnitRow, UnitUsage, UnitsPorts,
};

use crate::announcer::{Announcer, Domain};
use crate::write_count::WriteCount;

/// The Units ports the Units module and the General page share.
pub type DesktopPorts = Rc<RefCell<LedgerBackedPorts>>;

/// The shared Units store's Units, borrowed for each call.
struct SharedUnits(SharedUnitsStore);

impl UnitRepository for SharedUnits {
    fn units(&self) -> Vec<UnitRow> {
        self.0.borrow().units().units()
    }

    fn upsert_unit(&mut self, unit: UnitRow) {
        let unread = lib_units::fake::InMemoryUnitUsage::default();
        self.0
            .borrow_mut()
            .split_mut(&unread)
            .units
            .upsert_unit(unit);
    }

    fn delete_unit(&mut self, id: UnitId) {
        let unread = lib_units::fake::InMemoryUnitUsage::default();
        self.0.borrow_mut().split_mut(&unread).units.delete_unit(id);
    }
}

/// The shared Units store's stored Price Sources, borrowed for each call.
struct SharedPriceSources(SharedUnitsStore);

impl PriceSourceRepository for SharedPriceSources {
    fn price_source_records(&self) -> Vec<PriceSourceRecord> {
        self.0.borrow().price_sources().price_source_records()
    }

    fn upsert_price_source(&mut self, record: PriceSourceRecord) {
        let unread = lib_units::fake::InMemoryUnitUsage::default();
        self.0
            .borrow_mut()
            .split_mut(&unread)
            .price_sources
            .upsert_price_source(record);
    }

    fn delete_price_source(&mut self, id: PriceSourceId) {
        let unread = lib_units::fake::InMemoryUnitUsage::default();
        self.0
            .borrow_mut()
            .split_mut(&unread)
            .price_sources
            .delete_price_source(id);
    }
}

/// The shared Units store's base Unit, borrowed for each call.
struct SharedBaseUnit(SharedUnitsStore);

impl BaseUnitRepository for SharedBaseUnit {
    fn base_unit(&self) -> UnitId {
        self.0.borrow().base_unit().base_unit()
    }

    fn set_base_unit(&mut self, id: UnitId) {
        let unread = lib_units::fake::InMemoryUnitUsage::default();
        self.0
            .borrow_mut()
            .split_mut(&unread)
            .base_unit
            .set_base_unit(id);
    }
}

/// `store`'s Units, Price Sources and base Unit, with what names each Unit and the holdings of
/// each read from the other domains' stores through `reads`.
pub struct LedgerBackedPorts {
    store: SharedUnitsStore,
    units: SharedUnits,
    price_sources: SharedPriceSources,
    base_unit: SharedBaseUnit,
    reads: StoredUnitReads,
    writes: WriteCount,
    /// The write count the last announcement saw, so a write is announced once.
    announced: Cell<u64>,
}

impl LedgerBackedPorts {
    pub fn shared(store: SharedUnitsStore, reads: StoredUnitReads) -> DesktopPorts {
        Rc::new(RefCell::new(Self {
            units: SharedUnits(store.clone()),
            price_sources: SharedPriceSources(store.clone()),
            base_unit: SharedBaseUnit(store.clone()),
            store,
            reads,
            writes: WriteCount::default(),
            announced: Cell::new(0),
        }))
    }

    /// Whether these ports handed a mutable store out since the last ask.
    fn take_written(&self) -> bool {
        let writes = self.writes.get();
        writes != self.announced.replace(writes)
    }
}

impl UnitsPorts for LedgerBackedPorts {
    fn units(&self) -> &dyn UnitRepository {
        &self.units
    }

    fn price_sources(&self) -> &dyn PriceSourceRepository {
        &self.price_sources
    }

    fn base_unit(&self) -> &dyn BaseUnitRepository {
        &self.base_unit
    }

    fn usage(&self) -> &dyn UnitUsage {
        &self.reads
    }

    fn holdings(&self) -> &dyn UnitHoldings {
        &self.reads
    }

    fn split_mut(&mut self) -> PortsMut<'_> {
        self.writes.bump();
        PortsMut {
            units: &mut self.units,
            price_sources: &mut self.price_sources,
            base_unit: &mut self.base_unit,
            usage: &self.reads,
        }
    }
}

/// The same ports as the store they share, for the Unit Deletion adapter, which writes it with the
/// Institutions store in one delete (#903), and the Ledger page's base Unit.
impl lib_database::UnitsStore for LedgerBackedPorts {
    fn units(&self) -> &dyn UnitRepository {
        &self.units
    }

    fn price_sources(&self) -> &dyn PriceSourceRepository {
        &self.price_sources
    }

    fn base_unit(&self) -> &dyn BaseUnitRepository {
        &self.base_unit
    }

    fn split_mut<'a>(&'a mut self, usage: &'a dyn UnitUsage) -> PortsMut<'a> {
        self.writes.bump();
        PortsMut {
            units: &mut self.units,
            price_sources: &mut self.price_sources,
            base_unit: &mut self.base_unit,
            usage,
        }
    }

    fn reload(&mut self) -> lib_database::Result<()> {
        self.store.borrow_mut().reload()
    }
}

/// Prices every Unit in itself, so none is quoted in another and any can be deleted; for a UI test
/// that deletes a seeded Unit.
#[doc(hidden)]
pub fn unquote_for_test(ports: &DesktopPorts, announcer: &Entity<Announcer>, cx: &mut App) {
    {
        let mut ports = ports.borrow_mut();
        let units = UnitsPorts::units(&*ports).units();
        for mut unit in units {
            if unit.link.quote != unit.id {
                unit.link.quote = unit.id;
                UnitsPorts::split_mut(&mut *ports).units.upsert_unit(unit);
            }
        }
    }
    announce(ports, announcer, cx);
}

/// Puts the ports back to the rows as stored, after a write that named the Units failed,
/// announcing `Domain::Units` so every Units view model is rebuilt (ADR-0046).
///
/// # Errors
/// Returns the error reading the rows back; the ports are left as they were.
pub fn reload(
    ports: &DesktopPorts,
    announcer: &Entity<Announcer>,
    cx: &mut App,
) -> lib_database::Result<()> {
    lib_database::UnitsStore::reload(&mut *ports.borrow_mut())?;
    announcer.update(cx, |announcer, cx| announcer.announce(Domain::Units, cx));
    Ok(())
}

/// Announces `Domain::Units` when the ports were written since the last announcement, so the
/// Accounts, Inventory, Bills, Budgets, Transactions and the Dashboard see a write at once. Run
/// before and after every Units event and Dialog confirm. Returns whether it announced.
pub fn announce(ports: &DesktopPorts, announcer: &Entity<Announcer>, cx: &mut App) -> bool {
    let written = ports.borrow().take_written();
    if written {
        announcer.update(cx, |announcer, cx| announcer.announce(Domain::Units, cx));
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ports() -> (SharedUnitsStore, DesktopPorts) {
        let storage = crate::shell::router::LedgerStores::build(
            lib_database::Stores::fake(lib_database::FakeStores::sample(
                chrono::NaiveDate::from_ymd_opt(2026, 10, 30).expect("a valid date"),
            ))
            .0,
        )
        .stores;
        let store = storage.units.clone();
        let ports = LedgerBackedPorts::shared(
            store.clone(),
            StoredUnitReads {
                accounts: storage.accounts.clone(),
                transactions: storage.transactions.clone(),
                bills: storage.bills.borrow().store(),
                budgets: storage.budgets.clone(),
                inventory: storage.inventory.clone(),
                institutions: storage.institutions.clone(),
            },
        );
        (store, ports)
    }

    #[test]
    fn a_write_through_the_ports_lands_in_the_shared_store_and_is_noted_once() {
        let (store, ports) = ports();
        assert!(!ports.borrow().take_written(), "nothing written yet");

        let mut aud = store
            .borrow()
            .units()
            .unit(UnitId::SEED_AUD)
            .expect("the sample's aud");
        aud.code = "aud2".into();
        UnitsPorts::split_mut(&mut *ports.borrow_mut())
            .units
            .upsert_unit(aud);

        assert_eq!(
            store.borrow().unit_code(UnitId::SEED_AUD).as_deref(),
            Some("aud2")
        );
        assert!(ports.borrow().take_written());
        assert!(!ports.borrow().take_written(), "a write is announced once");
    }
}
