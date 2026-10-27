//! The Institutions ports as the Desktop holds them: the store `shell::router::build` is given,
//! and the step that keeps the Ledger's read copy of the Institutions in line after an Institutions
//! write. The write is committed by the input it was made in (`crate::input::send`), not here.
//!
//! The ports are one value shared two ways: the Institutions module's Presenters read and write
//! them as [`InstitutionsPorts`], and the Ledger store reads the rows back from them to refresh its
//! copy and reloads them after a failed write.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{App, Entity};
use lib_database::InstitutionsStore;
use lib_institutions::{InstitutionRepository, InstitutionsPorts};
use lib_ledger::Ledger;

use crate::ledger_store::LedgerStore;

/// The Institutions ports the Institutions module and the Ledger store share.
pub type DesktopPorts = Rc<RefCell<LedgerBackedPorts>>;

/// `store`'s Institutions.
pub struct LedgerBackedPorts {
    store: Box<dyn InstitutionsStore>,
}

impl LedgerBackedPorts {
    pub fn shared(store: Box<dyn InstitutionsStore>) -> DesktopPorts {
        Rc::new(RefCell::new(Self { store }))
    }

    /// `ledger`'s own Institutions in memory, for a test that needs no storage.
    pub fn in_memory(ledger: &Ledger) -> DesktopPorts {
        Self::shared(Box::new(lib_institutions::fake::InMemoryInstitutions {
            institutions: ledger.institutions().to_vec(),
        }))
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
        let ports = LedgerBackedPorts::in_memory(&ledger);
        let ports = ports.borrow();

        assert_eq!(
            InstitutionsPorts::institutions(&*ports).institutions(),
            ledger.institutions()
        );
    }
}
