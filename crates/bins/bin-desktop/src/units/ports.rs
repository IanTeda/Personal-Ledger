//! The Units ports as the Desktop holds them: the store `shell::router::build` is given, with
//! `UnitUsage` and `UnitHoldings` read from the Accounts, Transactions, Bills, Budgets, Inventory
//! and Institutions stores through `lib-database`'s [`StoredUnitReads`] (#993), and the step that
//! keeps the Ledger's read copy of the Units in line after a Units write. The write is committed
//! by the input it was made in (`crate::input::send`), not here.
//!
//! Usage and holdings are counted in the stores on each ask, whether they are the adapters or
//! `Stores::fake`'s in-memory ones, so nothing is copied from the Ledger.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{App, Entity};
use lib_database::StoredUnitReads;
use lib_units::{
    BaseUnitRepository, PortsMut, PriceSourceRepository, UnitHoldings, UnitRepository, UnitUsage,
    UnitsPorts,
};

use crate::ledger_store::LedgerStore;

/// The Units ports the Units module and the General page share.
pub type DesktopPorts = Rc<RefCell<LedgerBackedPorts>>;

/// `store`'s Units, Price Sources and base Unit, with what names each Unit and the holdings of
/// each read from the other domains' stores through `reads`.
pub struct LedgerBackedPorts {
    store: Box<dyn lib_database::UnitsStore>,
    reads: StoredUnitReads,
}

impl LedgerBackedPorts {
    pub fn shared(
        store: Box<dyn lib_database::UnitsStore>,
        reads: StoredUnitReads,
    ) -> DesktopPorts {
        Rc::new(RefCell::new(Self { store, reads }))
    }
}

impl UnitsPorts for LedgerBackedPorts {
    fn units(&self) -> &dyn UnitRepository {
        self.store.units()
    }

    fn price_sources(&self) -> &dyn PriceSourceRepository {
        self.store.price_sources()
    }

    fn base_unit(&self) -> &dyn BaseUnitRepository {
        self.store.base_unit()
    }

    fn usage(&self) -> &dyn UnitUsage {
        &self.reads
    }

    fn holdings(&self) -> &dyn UnitHoldings {
        &self.reads
    }

    fn split_mut(&mut self) -> PortsMut<'_> {
        self.store.split_mut(&self.reads)
    }
}

/// The same ports as the store they wrap, for the Unit Deletion adapter, which writes it with the
/// Institutions store in one delete (#903).
impl lib_database::UnitsStore for LedgerBackedPorts {
    fn units(&self) -> &dyn UnitRepository {
        self.store.units()
    }

    fn price_sources(&self) -> &dyn PriceSourceRepository {
        self.store.price_sources()
    }

    fn base_unit(&self) -> &dyn BaseUnitRepository {
        self.store.base_unit()
    }

    fn split_mut<'a>(&'a mut self, usage: &'a dyn UnitUsage) -> PortsMut<'a> {
        self.store.split_mut(usage)
    }

    fn reload(&mut self) -> lib_database::Result<()> {
        self.store.reload()
    }
}

/// Prices every Unit in itself, so none is quoted in another and any can be deleted; for a UI test
/// that deletes a seeded Unit.
#[doc(hidden)]
pub fn unquote_for_test(ports: &DesktopPorts, ledger: &Entity<LedgerStore>, cx: &mut App) {
    {
        let mut ports = ports.borrow_mut();
        let units = ports.units().units();
        for mut unit in units {
            if unit.link.quote != unit.id {
                unit.link.quote = unit.id;
                ports.split_mut().units.upsert_unit(unit);
            }
        }
    }
    refresh(ports, ledger, cx);
}

/// Puts the ports back to the rows as stored, after a write that named the Units failed, and
/// brings the Ledger read copy in line, announcing `Domain::Units` so every Units view model is
/// rebuilt (ADR-0046).
///
/// # Errors
/// Returns the error reading the rows back; the ports and the read copy are left as they were.
pub fn reload(
    ports: &DesktopPorts,
    ledger: &Entity<LedgerStore>,
    cx: &mut App,
) -> lib_database::Result<()> {
    ports.borrow_mut().store.reload()?;
    refresh(ports, ledger, cx);
    Ok(())
}

/// Brings the Ledger read copy in line with the ports, announcing `Domain::Units` when it changed,
/// Run before and after every Units event and Dialog confirm, so Bills, Budgets, Transactions and
/// the Dashboard see a write at once.
pub fn refresh(ports: &DesktopPorts, ledger: &Entity<LedgerStore>, cx: &mut App) {
    let (units, base, records) = {
        let ports = ports.borrow();
        (
            ports.units().units(),
            ports.base_unit().base_unit(),
            ports.price_sources().price_source_records(),
        )
    };
    ledger.update(cx, |store, cx| {
        store.refresh_units(units, base, records, cx)
    });
}
