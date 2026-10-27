//! The in-memory ports and the stub seed, for tests and UI-first work. Each holds its rows in a
//! plain field so a test can set up or read state directly. [`RecordingRouter`] is a Router that
//! only records what it was asked.

use std::cell::RefCell;
use std::rc::Rc;

use lib_toast::{Notice, Notify};

use crate::protocols::{
    InstitutionsDialog, InstitutionsOutcome, InstitutionsPorts, InstitutionsRouter, SharedPorts,
};
use crate::{Institution, InstitutionRepository, default_institutions};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryInstitutions {
    pub institutions: Vec<Institution>,
}

impl InMemoryInstitutions {
    /// The stub seed: the stub Ledger's Institutions (see [`default_institutions`]). Ids match
    /// `lib-ledger`'s stub, so its Accounts still resolve their Institutions.
    pub fn stub() -> Self {
        Self {
            institutions: default_institutions(),
        }
    }
}

impl InstitutionRepository for InMemoryInstitutions {
    fn institutions(&self) -> Vec<Institution> {
        self.institutions.clone()
    }

    fn add_institution(&mut self, institution: Institution) {
        self.institutions.push(institution);
    }
}

/// Every Institutions port in memory.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InMemoryPorts {
    pub institutions: InMemoryInstitutions,
}

impl InMemoryPorts {
    /// The stub Institutions.
    pub fn stub() -> Self {
        Self {
            institutions: InMemoryInstitutions::stub(),
        }
    }

    /// These ports shared, as the Presenters hold them, with a handle a test reads them back
    /// through.
    pub fn shared(self) -> (Rc<RefCell<Self>>, SharedPorts) {
        let ports = Rc::new(RefCell::new(self));
        let shared: SharedPorts = ports.clone();
        (ports, shared)
    }
}

impl InstitutionsPorts for InMemoryPorts {
    fn institutions(&self) -> &dyn InstitutionRepository {
        &self.institutions
    }

    fn institutions_mut(&mut self) -> &mut dyn InstitutionRepository {
        &mut self.institutions
    }
}

/// What a Presenter asked its Router for, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouterCall {
    OpenDialog(InstitutionsDialog),
    CloseDialog(Option<InstitutionsOutcome>),
    Notice(Notice),
}

/// A Router that does nothing but record each call, in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecordingRouter {
    pub calls: Vec<RouterCall>,
}

impl RecordingRouter {
    /// The calls made since the last `take`.
    pub fn take(&mut self) -> Vec<RouterCall> {
        std::mem::take(&mut self.calls)
    }
}

impl InstitutionsRouter for RecordingRouter {
    /// Always opens: the recording Router builds no Presenter that could fail.
    fn open_dialog(&mut self, dialog: InstitutionsDialog) -> bool {
        self.calls.push(RouterCall::OpenDialog(dialog));
        true
    }

    fn close_dialog(&mut self, outcome: Option<InstitutionsOutcome>) {
        self.calls.push(RouterCall::CloseDialog(outcome));
    }
}

impl Notify for RecordingRouter {
    fn notify(&mut self, notice: Notice) {
        self.calls.push(RouterCall::Notice(notice));
    }
}
