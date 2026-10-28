//! The port Unit delete writes through.
//!
//! [`UnitDeletion`] is the module's one atomic port: a Unit goes with the base's move, when it was
//! the base, and the new Usual unit of every Institution that named it, written together or not at
//! all. It is synchronous (ADR-0046): an adapter serves its reads from memory and queues every
//! write in the batch every adapter shares, so the bin's one commit after the call lands them as
//! one transaction, and a failed save is the adapter's to report. `fake` implements it in memory.
//!
//! It serves the reads the Interactor checks as owned values, rather than the Interactor taking the
//! Units and Institutions ports beside it: the adapter holds stores the Units and Institutions
//! modules share, so an Interactor given them all would borrow one twice (#779). What names the
//! Unit in the domains that aren't written, the Accounts, Bill Plans, Budgets and Locations, comes
//! through `lib-units`' [`UnitUsage`](lib_units::UnitUsage) read port instead, which the Desktop
//! answers from its Ledger copy.
//!
//! The Dialog closes through `lib-units`' [`UnitsRouter`](lib_units::UnitsRouter), as every Units
//! Dialog does, so the Units page hears of the delete as it would of its own.

use std::cell::RefCell;
use std::rc::Rc;

use lib_core::UnitId;
use lib_institutions::Institution;
use lib_units::UnitRow;

/// Everything deleting one Unit writes, as one write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitRemoval {
    pub unit: UnitId,
    /// The Unit that takes over as base, when the Unit was the base.
    pub new_base: Option<UnitId>,
    /// Every Institution that named the Unit as its Usual unit, with the Usual unit it has now.
    pub reassigned: Vec<Institution>,
}

/// The rows Unit delete checks, and the one write that removes a Unit.
pub trait UnitDeletion {
    /// Every Unit, in no particular order.
    fn units(&self) -> Vec<UnitRow>;

    fn base_unit(&self) -> UnitId;

    /// The Institutions whose Usual unit is `unit`, in the Ledger's order.
    fn institutions_naming(&self, unit: UnitId) -> Vec<Institution>;

    /// Writes `removal` as one write: the Institutions' new Usual units, the base's move, then the
    /// Unit's removal, so nothing names the Unit when it goes. All land or none do.
    fn remove_unit(&mut self, removal: UnitRemoval);
}

/// Those of `institutions` whose Usual unit is `unit`, in order: what every store answers
/// [`UnitDeletion::institutions_naming`] with.
pub fn naming(institutions: Vec<Institution>, unit: UnitId) -> Vec<Institution> {
    institutions
        .into_iter()
        .filter(|institution| institution.usual_unit == Some(unit))
        .collect()
}

/// The deletion port as the Delete unit Dialog holds it, shared with the Client that commits its
/// writes. Single-threaded, as both Clients' UI threads are.
pub type SharedUnitDeletion = Rc<RefCell<dyn UnitDeletion>>;

/// What a delete removed and reassigned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitDeleted {
    pub unit: UnitRow,
    /// How many Institutions took another Unit as their Usual unit.
    pub reassigned: usize,
    /// How many were left with none.
    pub cleared: usize,
}
