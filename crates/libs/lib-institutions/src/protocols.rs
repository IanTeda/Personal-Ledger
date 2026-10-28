//! The ports Institutions reach storage and the other domains through, in the shape of
//! `lib-payees`'. They are synchronous (ADR-0046): an adapter serves reads from memory and queues
//! each write, so a write never fails here; a failed save is the adapter's to report. `fake`
//! implements them in memory.
//!
//! The Presenters reach every port together, through [`InstitutionsPorts`], and ask for navigation
//! through [`InstitutionsRouter`], which each Client implements. What names an Institution is the
//! Accounts', so [`InstitutionUsage`] is a read port each Client answers from its Accounts.

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

    /// Writes `institution`: a new id is a new row after every row already there, a known id
    /// replaces that row where it stands.
    fn upsert_institution(&mut self, institution: Institution);

    /// Removes Institution `id`; an unknown id is ignored. It does not check usage: the
    /// `DeleteInstitution` Interactor refuses an Institution in use, and storage's foreign key
    /// backs it.
    fn delete_institution(&mut self, id: InstitutionId);
}

/// What names one Institution: the names of the Accounts that do, in the Accounts' order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstitutionUse {
    pub accounts: Vec<String>,
}

impl InstitutionUse {
    /// Whether any Account names the Institution, so it can't be deleted.
    pub fn is_used(&self) -> bool {
        !self.accounts.is_empty()
    }
}

/// What names an Institution, read when the Delete Dialog opens and again when it confirms.
pub trait InstitutionUsage {
    fn institution_usage(&self, id: InstitutionId) -> InstitutionUse;
}

/// Every Institutions port, as one value the Presenters share.
pub trait InstitutionsPorts {
    fn institutions(&self) -> &dyn InstitutionRepository;

    fn institutions_mut(&mut self) -> &mut dyn InstitutionRepository;

    fn usage(&self) -> &dyn InstitutionUsage;
}

/// The ports as the page and its Dialog hold them: one set, shared, so an Institution the Dialog
/// adds is on the page as soon as the Dialog closes. Single-threaded, as both Clients' UI threads
/// are.
pub type SharedPorts = Rc<RefCell<dyn InstitutionsPorts>>;

/// An Institutions Dialog, as the page asks the Router to open it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstitutionsDialog {
    AddInstitution,
    /// The Add form for one Institution, pre-filled.
    EditInstitution(InstitutionId),
    /// Delete for one Institution: typed-back name when no Account names it, a refusal listing
    /// the Accounts when one does.
    DeleteInstitution(InstitutionId),
}

/// What a confirmed Institutions Dialog did, handed to the page when the Dialog closes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstitutionsOutcome {
    pub id: InstitutionId,
    /// The Institution's name as written, or as it was when deleted.
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
