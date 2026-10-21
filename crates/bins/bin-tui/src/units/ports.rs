//! The Units ports as the TUI holds them: the store `build_shell` is given, with `UnitUsage` and
//! `UnitHoldings` answered by the stub `Ledger` (#746's halfway shims, as on the Desktop). The
//! TUI's other Views still draw their own fixtures, so no read copy needs refreshing after a write.

use std::cell::RefCell;
use std::rc::Rc;

use lib_ledger::Ledger;
use lib_units::{
    BaseUnitRepository, PortsMut, PriceSourceRepository, UnitHoldings, UnitRepository, UnitUsage,
    UnitsPorts,
};

use super::store::UnitsStore;

/// The Units ports the Units View and its Dialogs share.
pub type TuiPorts = Rc<RefCell<LedgerBackedPorts>>;

/// `store`'s Units, Price Sources and base Unit, with usage and holdings read from `ledger`.
pub struct LedgerBackedPorts {
    store: Box<dyn UnitsStore>,
    ledger: Ledger,
}

impl LedgerBackedPorts {
    pub fn shared(store: Box<dyn UnitsStore>, ledger: Ledger) -> TuiPorts {
        Rc::new(RefCell::new(Self { store, ledger }))
    }

    /// Ends the Interactor call just made, if any, as one transaction.
    pub fn commit(&mut self) {
        self.store.commit();
    }

    /// Puts the rows back as stored, after a write that named the Units failed (ADR-0046).
    ///
    /// # Errors
    /// Returns the error reading the rows back; the rows in memory are left as they were.
    pub fn reload(&mut self) -> lib_database::Result<()> {
        self.store.reload()
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
        &self.ledger
    }

    fn holdings(&self) -> &dyn UnitHoldings {
        &self.ledger
    }

    fn split_mut(&mut self) -> PortsMut<'_> {
        self.store.split_mut(&self.ledger)
    }
}
