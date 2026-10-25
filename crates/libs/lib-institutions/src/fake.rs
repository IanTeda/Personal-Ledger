//! The in-memory port and the stub seed, for tests and UI-first work. It holds its rows in a plain
//! field so a test can set up or read state directly.

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
