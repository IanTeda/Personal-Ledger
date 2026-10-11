//! The shared Institutions store (ADR-0032).

use gpui::{Context, EventEmitter};
use lib_institutions::{InstitutionRow, InstitutionService};

use crate::ledger_data::LedgerDataEvent;

/// The shared Institutions table. Settings edits it, and the Accounts form reads it.
pub struct InstitutionsStore {
    service: InstitutionService,
}

impl EventEmitter<LedgerDataEvent> for InstitutionsStore {}

impl InstitutionsStore {
    pub fn new(institutions: Vec<InstitutionRow>) -> Self {
        Self {
            service: InstitutionService::new(institutions),
        }
    }

    pub fn institutions(&self) -> &[InstitutionRow] {
        self.service.institutions()
    }

    pub fn add(&mut self, cx: &mut Context<'_, Self>, name: String, account_type: String) {
        self.service.add(name, account_type);
        cx.emit(LedgerDataEvent::Changed);
    }
}
