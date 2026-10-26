//! The Units ports as the Desktop holds them: the store `shell::router::build` is given, with
//! `UnitHoldings` and the Accounts half of `UnitUsage` answered from the Accounts store (#853), the
//! Bill Plans and Budgets halves from the Ledger read copy (#746's halfway shims), and the step that
//! keeps that read copy in line after a Units write. The write is committed by the input it was
//! made in (`crate::input::send`), not here.
//!
//! Usage and holdings are copied before each Units event, since a port can't reach the
//! `LedgerStore` Entity without a gpui context. The Ledger's halves go with the shims, when Bills
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
use lib_database::{SharedAccountsStore, UnitsStore};

/// The Units ports the Units module and the General page share.
pub type DesktopPorts = Rc<RefCell<LedgerBackedPorts>>;

/// `store`'s Units, Price Sources and base Unit, with holdings and the Accounts naming each Unit
/// copied from `accounts`, and the Bill Plans and Budgets naming it from the Ledger.
pub struct LedgerBackedPorts {
    store: Box<dyn UnitsStore>,
    accounts: SharedAccountsStore,
    usage: InMemoryUnitUsage,
    holdings: InMemoryHoldings,
}

impl LedgerBackedPorts {
    pub fn shared(store: Box<dyn UnitsStore>, accounts: SharedAccountsStore) -> DesktopPorts {
        Rc::new(RefCell::new(Self {
            store,
            accounts,
            usage: InMemoryUnitUsage::default(),
            holdings: InMemoryHoldings::default(),
        }))
    }

    /// Copies what names each Unit from the Accounts store and `ledger`, so the next Interactor and
    /// view model read it.
    fn copy_usage(&mut self, ledger: &Ledger) {
        crate::settings::accounts::ports::recount(&self.accounts, ledger);
        let accounts = self.accounts.borrow();
        let ids: Vec<UnitId> = self
            .store
            .units()
            .units()
            .into_iter()
            .map(|unit| unit.id)
            .collect();
        self.usage.uses = ids
            .iter()
            .map(|id| {
                let usage = lib_units::UnitUse {
                    accounts: accounts.unit_accounts(*id),
                    ..ledger.unit_usage(*id)
                };
                (*id, usage)
            })
            .collect();
        self.holdings.by_unit = ids
            .iter()
            .map(|id| (*id, accounts.holdings(*id)))
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

/// Copies what names each Unit from `ledger` into `ports`, for a view model drawn outside an
/// event.
pub fn copy_usage(ports: &DesktopPorts, ledger: &Ledger) {
    ports.borrow_mut().copy_usage(ledger);
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
/// and copies usage and holdings back for the next Units event. Run before and after every Units
/// event and Dialog confirm, so Bills, Budgets, Transactions and the Dashboard see a write at once.
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
    copy_usage(ports, ledger.read(cx).ledger());
}
