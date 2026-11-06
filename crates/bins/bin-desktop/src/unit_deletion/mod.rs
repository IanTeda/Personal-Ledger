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

use crate::announcer::Announcer;

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

/// Announces the Units and, when they were written, the Institutions after a confirm, so the Units
/// and Institutions pages, the Accounts and the Dashboard see the delete at once. A Unit delete
/// writes the Institutions that name the Unit in the same step as the Units, and these ports can't
/// see the Institutions' write count from here, so a Units write stands for it. The confirm's
/// input commits it.
pub(crate) fn refresh(
    units: &crate::units::DesktopPorts,
    announcer: &Entity<Announcer>,
    cx: &mut App,
) {
    if crate::units::ports::announce(units, announcer, cx) {
        crate::institutions::ports::announce(announcer, cx);
    }
}
