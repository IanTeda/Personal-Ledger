//! The Institutions ports as the Desktop holds them: the store `shell::router::build` is given,
//! with the Units copied from the Ledger read copy (a halfway shim until Units answer the Default
//! unit select through their own port), and the step that keeps the Ledger's read copy of the
//! Institutions in line after an Institutions write. The write is committed by the input it was
//! made in (`crate::input::send`), not here.
//!
//! The ports are one value shared two ways: the Institutions module's Presenters read and write
//! them as [`InstitutionsPorts`], and the Ledger store reads the rows back from them to refresh its
//! copy and reloads them after a failed write.
//!
//! The Units are copied from the Ledger before each Institutions event, since a port can't reach
//! the `LedgerStore` Entity without a gpui context.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{App, Entity};
use lib_database::InstitutionsStore;
use lib_institutions::fake::InMemoryInstitutionUnits;
use lib_institutions::{InstitutionRepository, InstitutionUnits, InstitutionsPorts};
use lib_ledger::Ledger;

use crate::ledger_store::LedgerStore;

/// The Institutions ports the Institutions module and the Ledger store share.
pub type DesktopPorts = Rc<RefCell<LedgerBackedPorts>>;

/// `store`'s Institutions, with the Units a new one may default to copied from the Ledger.
pub struct LedgerBackedPorts {
    store: Box<dyn InstitutionsStore>,
    units: InMemoryInstitutionUnits,
}

impl LedgerBackedPorts {
    pub fn shared(store: Box<dyn InstitutionsStore>) -> DesktopPorts {
        Rc::new(RefCell::new(Self {
            store,
            units: InMemoryInstitutionUnits::default(),
        }))
    }

    /// `ledger`'s own Institutions in memory, for a test that needs no storage.
    pub fn in_memory(ledger: &Ledger) -> DesktopPorts {
        let ports = Self::shared(Box::new(lib_institutions::fake::InMemoryInstitutions {
            institutions: ledger.institutions().to_vec(),
        }));
        ports.borrow_mut().copy_from_ledger(ledger);
        ports
    }

    fn copy_from_ledger(&mut self, ledger: &Ledger) {
        self.units.codes = ledger
            .units()
            .iter()
            .map(|unit| unit.code.clone())
            .collect();
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

    fn units(&self) -> &dyn InstitutionUnits {
        &self.units
    }
}

/// Copies the Units a new Institution may default to from `ledger` into `ports`, for the next
/// Institutions event and a view model drawn outside one.
pub fn copy_from_ledger(ports: &DesktopPorts, ledger: &Ledger) {
    ports.borrow_mut().copy_from_ledger(ledger);
}

/// Brings the Ledger read copy of the Institutions in line with the ports, announcing
/// `Domain::Institutions` when it changed, and copies the Units back for the next Institutions
/// event. Run before and after every Institutions event and Dialog confirm, so Accounts see a write
/// at once.
pub fn refresh(ports: &DesktopPorts, ledger: &Entity<LedgerStore>, cx: &mut App) {
    ledger.update(cx, |store, cx| store.refresh_institutions(cx));
    copy_from_ledger(ports, ledger.read(cx).ledger());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_ports_hold_the_ledgers_institutions_and_offer_its_units() {
        let ledger = Ledger::stub();
        let ports = LedgerBackedPorts::in_memory(&ledger);
        let ports = ports.borrow();

        assert_eq!(
            InstitutionsPorts::institutions(&*ports).institutions(),
            ledger.institutions()
        );
        assert_eq!(
            ports.units().unit_codes().first().map(String::as_str),
            Some("aud")
        );
    }
}
