//! The Units ports as the Desktop holds them: the store `build_shell` is given, with `UnitUsage`
//! and `UnitHoldings` answered from the Ledger read copy (#746's halfway shims), and the step that
//! commits a Units write and keeps that read copy in line after it.
//!
//! Usage and holdings are copied from the Ledger before each Units event, since a port can't reach
//! the `LedgerStore` Entity without a gpui context. They go with the shims, when Accounts, Bills
//! and Budgets answer them through their own ports.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use gpui::{App, Entity};
use lib_core::UnitId;
use lib_ledger::Ledger;
use lib_units::fake::{InMemoryHoldings, InMemoryUnitUsage};
use lib_units::{
    BaseUnitRepository, PortsMut, PriceSourceRepository, UnitHoldings, UnitRepository, UnitUsage,
    UnitsPorts,
};

use crate::ledger_store::LedgerStore;
use lib_database::UnitsStore;

/// The Units ports the Units module and the General page share.
pub type DesktopPorts = Rc<RefCell<LedgerBackedPorts>>;

/// `store`'s Units, Price Sources and base Unit, with usage and holdings copied from the Ledger.
pub struct LedgerBackedPorts {
    store: Box<dyn UnitsStore>,
    usage: InMemoryUnitUsage,
    holdings: InMemoryHoldings,
}

impl LedgerBackedPorts {
    pub fn shared(store: Box<dyn UnitsStore>) -> DesktopPorts {
        Rc::new(RefCell::new(Self {
            store,
            usage: InMemoryUnitUsage::default(),
            holdings: InMemoryHoldings::default(),
        }))
    }

    /// Copies what names each Unit from `ledger`, so the next Interactor and view model read it.
    fn copy_usage(&mut self, ledger: &Ledger) {
        let ids: Vec<UnitId> = self
            .store
            .units()
            .units()
            .into_iter()
            .map(|unit| unit.id)
            .collect();
        self.usage.uses = ids.iter().map(|id| (*id, ledger.unit_usage(*id))).collect();
        self.holdings.by_unit = ids
            .iter()
            .map(|id| (*id, ledger.holdings(*id)))
            .collect::<HashMap<_, _>>();
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
        &self.usage
    }

    fn holdings(&self) -> &dyn UnitHoldings {
        &self.holdings
    }

    fn split_mut(&mut self) -> PortsMut<'_> {
        self.store.split_mut(&self.usage)
    }
}

/// Copies what names each Unit from `ledger` into `ports`, for a view model drawn outside an
/// event.
pub fn copy_usage(ports: &DesktopPorts, ledger: &Ledger) {
    ports.borrow_mut().copy_usage(ledger);
}

/// Commits the Interactor call just made, if any, then brings the Ledger read copy in line with
/// the ports, announcing `Domain::Units` when it changed, and copies usage and holdings back for the
/// next Units event. Run after every Units event and Dialog confirm, so a write is saved as one
/// transaction and Bills, Budgets, Transactions and the Dashboard see it at once.
pub fn sync(ports: &DesktopPorts, ledger: &Entity<LedgerStore>, cx: &mut App) {
    ports.borrow_mut().store.commit();
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

/// Brings the Ledger read copy in line with the ports and copies usage and holdings back.
fn refresh(ports: &DesktopPorts, ledger: &Entity<LedgerStore>, cx: &mut App) {
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
    copy_usage(ports, ledger.read(cx).ledger());
}
