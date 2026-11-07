//! `lib-institutions`' ports answered over the stores: the Institutions from the Institutions
//! store, and the Accounts naming each and the Units a Usual unit may name from the Accounts and
//! Units stores through [`StoredInstitutionReads`] (#994, #1052, #1084).

use lib_core::InstitutionId;
use lib_institutions::{
    Institution, InstitutionRepository, InstitutionUnits, InstitutionUsage, InstitutionsPorts,
};

use crate::adapter::store::{SharedInstitutionsStore, StoredInstitutionReads};

/// The shared Institutions store's Institutions, borrowed for each call: the store's guard can't
/// be lent out of its `RefCell`.
struct Institutions(SharedInstitutionsStore);

impl InstitutionRepository for Institutions {
    fn institutions(&self) -> Vec<Institution> {
        self.0.borrow().institutions().institutions()
    }

    fn upsert_institution(&mut self, institution: Institution) {
        self.0
            .borrow_mut()
            .institutions_mut()
            .upsert_institution(institution);
    }

    fn delete_institution(&mut self, id: InstitutionId) {
        self.0
            .borrow_mut()
            .institutions_mut()
            .delete_institution(id);
    }
}

/// [`lib_institutions::InstitutionsPorts`] over the shared Institutions store, and the reads over
/// the Accounts and Units stores. A write reaches the store's recorder, so the commit names the
/// Institutions in what the input wrote.
pub struct StoredInstitutionsPorts {
    institutions: Institutions,
    reads: StoredInstitutionReads,
}

impl StoredInstitutionsPorts {
    pub fn new(institutions: SharedInstitutionsStore, reads: StoredInstitutionReads) -> Self {
        Self {
            institutions: Institutions(institutions),
            reads,
        }
    }
}

impl InstitutionsPorts for StoredInstitutionsPorts {
    fn institutions(&self) -> &dyn InstitutionRepository {
        &self.institutions
    }

    fn institutions_mut(&mut self) -> &mut dyn InstitutionRepository {
        &mut self.institutions
    }

    fn usage(&self) -> &dyn InstitutionUsage {
        &self.reads
    }

    fn units(&self) -> &dyn InstitutionUnits {
        &self.reads
    }
}

#[cfg(test)]
mod tests {
    use lib_core::InstitutionId;

    use crate::ports::test_support::{fakes, file};
    use crate::{Domain, LedgerPorts};

    #[test]
    fn an_institutions_write_reports_the_institutions_written_on_either() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        for ledger in [fakes(), file(dir.path())] {
            let LedgerPorts {
                institutions,
                commit,
                ..
            } = ledger;
            let mut anz = institutions
                .borrow()
                .institutions()
                .institution(InstitutionId::SEED_ANZ)
                .expect("a sample Institution");
            anz.name.push_str(" renamed");

            institutions
                .borrow_mut()
                .institutions_mut()
                .upsert_institution(anz);

            assert_eq!(commit.commit().wrote, [Domain::Institutions]);
        }
    }

    #[test]
    fn the_usage_is_counted_in_the_accounts_store() {
        let ledger = fakes();

        let usage = ledger
            .institutions
            .borrow()
            .usage()
            .institution_usage(InstitutionId::SEED_ANZ);

        assert!(!usage.accounts.is_empty(), "the sample banks with ANZ");
        assert!(ledger.commit.commit().is_empty(), "a read writes nothing");
    }
}
