//! The ports Institutions reach storage and the other domains through, in the shape of
//! `lib-payees`'. They are synchronous (ADR-0046): an adapter serves reads from memory and queues
//! each write, so a write never fails here; a failed save is the adapter's to report. `fake`
//! implements them in memory.
//!
//! Only list and add exist: Institution edit and delete are a backlog ticket (#847). The
//! Presenters reach every port together, through [`InstitutionsPorts`], and ask for navigation
//! through [`InstitutionsRouter`], which each Client implements.

use std::cell::RefCell;
use std::rc::Rc;

use lib_core::InstitutionId;
use lib_toast::Notify;

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

/// The Units the Add Dialog offers as a Default unit, by code, in the Units' order. A read port, so
/// `lib-institutions` needn't depend on the Units crate. The choice is not saved: an Institution
/// has no unit yet.
pub trait InstitutionUnits {
    fn unit_codes(&self) -> Vec<String>;
}

/// Every Institutions port, as one value the Presenters share.
pub trait InstitutionsPorts {
    fn institutions(&self) -> &dyn InstitutionRepository;

    fn institutions_mut(&mut self) -> &mut dyn InstitutionRepository;

    fn units(&self) -> &dyn InstitutionUnits;
}

/// The ports as the page and its Dialog hold them: one set, shared, so an Institution the Dialog
/// adds is on the page as soon as the Dialog closes. Single-threaded, as both Clients' UI threads
/// are.
pub type SharedPorts = Rc<RefCell<dyn InstitutionsPorts>>;

/// An Institutions Dialog, as the page asks the Router to open it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstitutionsDialog {
    AddInstitution,
}

/// What a confirmed Institutions Dialog did, handed to the page when the Dialog closes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstitutionsOutcome {
    pub id: InstitutionId,
    /// The Institution's name as written.
    pub name: String,
}

/// Navigation out of the Institutions Presenters, implemented by each Client. The Client passes it
/// into each `handle` call rather than the Presenter holding it, since the Desktop can only open a
/// Dialog with the gpui context that call runs in. A Toast or status-line message goes to the
/// Shell through [`Notify`] (ADR-0048).
pub trait InstitutionsRouter: Notify {
    /// Builds `dialog`'s Presenter on the shared ports and hosts its View. Returns whether it
    /// opened.
    fn open_dialog(&mut self, dialog: InstitutionsDialog) -> bool;

    /// Closes the open Institutions Dialog. `outcome` is what its confirm did, `None` when it was
    /// cancelled; the Client hands it to the page as [`InstitutionsPageEvent::DialogClosed`].
    ///
    /// [`InstitutionsPageEvent::DialogClosed`]: crate::presenter::InstitutionsPageEvent::DialogClosed
    fn close_dialog(&mut self, outcome: Option<InstitutionsOutcome>);
}
