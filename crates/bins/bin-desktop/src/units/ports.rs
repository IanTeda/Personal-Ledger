//! The Units ports as the Desktop holds them: the ports `build_shell` is given, with `UnitUsage`
//! and `UnitHoldings` answered from the Ledger read copy (#746's halfway shims), and the step that
//! keeps that read copy in line after a Units write.
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

/// The Units ports the Units module and the General page share.
pub type DesktopPorts = Rc<RefCell<LedgerBackedPorts>>;

/// `store`'s Units, Price Sources and base Unit, with usage and holdings copied from the Ledger.
pub struct LedgerBackedPorts {
    store: Box<dyn UnitsPorts>,
    usage: InMemoryUnitUsage,
    holdings: InMemoryHoldings,
}

impl LedgerBackedPorts {
    pub fn shared(store: Box<dyn UnitsPorts>) -> DesktopPorts {
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
        let store = self.store.split_mut();
        PortsMut {
            units: store.units,
            price_sources: store.price_sources,
            base_unit: store.base_unit,
            usage: &self.usage,
        }
    }
}

/// Copies what names each Unit from `ledger` into `ports`, for a view model drawn outside an
/// event.
pub fn copy_usage(ports: &DesktopPorts, ledger: &Ledger) {
    ports.borrow_mut().copy_usage(ledger);
}

/// Brings the Ledger read copy in line with the ports, announcing `Domain::Units` when it changed,
/// then copies usage and holdings back for the next Units event. Run after every Units event and
/// Dialog confirm, so Bills, Budgets, Transactions and the Dashboard see a write at once.
pub fn sync(ports: &DesktopPorts, ledger: &Entity<LedgerStore>, cx: &mut App) {
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
