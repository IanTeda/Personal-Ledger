//! The Unit Deletion module's Desktop half (`docs/development/viper-architecture.md`, #903): the
//! Delete unit Dialog's View, which the Units page's Router opens for a delete. Every rule, every
//! piece of text and the Presenter are `lib-unit-deletion`'s; this folder turns gpui keys and
//! clicks into Presenter events and draws the view model.
//!
//! - `view/dialog/delete_unit.rs` holds the Presenter in an Entity in the Dialog host and draws it.
//!
//! Unit delete writes through [`deletion`], `lib-database`'s adapter over the Units ports and the
//! Institutions ports the two modules hold, so the Unit's removal, the base's move and the
//! Institutions' new Usual units join the one pending batch and land in one commit.

pub(crate) mod view;

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{App, Entity};
use lib_database::{SharedInstitutionsStore, SharedUnitsStore, UnitDeletionAdapter};
use lib_unit_deletion::SharedUnitDeletion;

use crate::ledger_store::LedgerStore;

pub use view::dialog::delete_unit::DeleteUnitDialog;

/// The deletion port over the stores `units` and `institutions` wrap.
pub fn deletion(
    units: &crate::units::DesktopPorts,
    institutions: &crate::institutions::ports::DesktopPorts,
) -> SharedUnitDeletion {
    let units: SharedUnitsStore = units.clone();
    let institutions: SharedInstitutionsStore = institutions.clone();
    Rc::new(RefCell::new(UnitDeletionAdapter::new(units, institutions)))
}

/// Brings the Ledger's Units and Institutions read copies in line after a confirm, so the Units
/// and Institutions pages, the Accounts and the Dashboard see the delete at once, and copies what
/// names each Unit back into the Units ports for the next event. The confirm's input commits it.
pub(crate) fn refresh(
    units: &crate::units::DesktopPorts,
    ledger: &Entity<LedgerStore>,
    cx: &mut App,
) {
    crate::units::ports::refresh(units, ledger, cx);
    crate::institutions::ports::refresh(ledger, cx);
    crate::units::ports::copy_usage(units, ledger.read(cx).ledger());
}
