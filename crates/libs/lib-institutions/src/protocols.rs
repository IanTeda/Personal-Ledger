//! The port Institutions reach storage through, in the shape of `lib-payees`'. It is synchronous
//! (ADR-0046): an adapter serves reads from memory and queues each write, so a write never fails
//! here; a failed save is the adapter's to report. `fake` implements it in memory.
//!
//! Only list and add exist: Institution edit and delete are a backlog ticket (#847). The page's
//! Presenter and its Router trait arrive with the page (#850).

use lib_core::InstitutionId;

use crate::Institution;

/// The Institutions on the Ledger.
pub trait InstitutionRepository {
    /// Every Institution, in the order they were added.
    fn institutions(&self) -> Vec<Institution>;

    fn institution(&self, id: InstitutionId) -> Option<Institution> {
        self.institutions()
            .into_iter()
            .find(|institution| institution.id == id)
    }

    /// Writes `institution` as a new row, after every row already there.
    fn add_institution(&mut self, institution: Institution);
}
