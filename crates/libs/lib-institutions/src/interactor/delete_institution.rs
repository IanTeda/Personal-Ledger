use lib_core::InstitutionId;

use crate::Institution;
use crate::protocols::{InstitutionUse, InstitutionsPorts};

/// Delete an Institution no Account names. Usage is read here, so the refusal is decided in one
/// place whatever the caller checked when it opened its Dialog: an Account may have come to name
/// the Institution since.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteInstitution {
    pub id: InstitutionId,
}

/// Why an Institution delete was refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DeleteInstitutionRefusal {
    /// The Institution is no longer on the Ledger.
    #[error("the institution is not on the Ledger")]
    NotFound,
    /// Accounts name it; carries them.
    #[error("the institution is used by {} accounts", .0.accounts.len())]
    InUse(InstitutionUse),
}

impl DeleteInstitution {
    /// Returns the Institution removed.
    pub fn run(
        self,
        ports: &mut dyn InstitutionsPorts,
    ) -> Result<Institution, DeleteInstitutionRefusal> {
        let outcome = self.apply(ports);
        match &outcome {
            Ok(_) => tracing::info!(institution_id = %self.id, "Deleted an Institution"),
            Err(refusal) => {
                tracing::info!(institution_id = %self.id, %refusal, "Refused to delete an Institution");
            }
        }
        outcome
    }

    fn apply(
        self,
        ports: &mut dyn InstitutionsPorts,
    ) -> Result<Institution, DeleteInstitutionRefusal> {
        let institution = ports
            .institutions()
            .institution(self.id)
            .ok_or(DeleteInstitutionRefusal::NotFound)?;
        let uses = ports.usage().institution_usage(self.id);
        if uses.is_used() {
            return Err(DeleteInstitutionRefusal::InUse(uses));
        }
        ports.institutions_mut().delete_institution(self.id);
        Ok(institution)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::InMemoryPorts;
    use crate::get;

    #[test]
    fn an_unused_institution_is_removed_and_returned() {
        let mut ports = InMemoryPorts::stub();

        let deleted = DeleteInstitution {
            id: InstitutionId::SEED_AMEX,
        }
        .run(&mut ports)
        .expect("no Account names it");

        assert_eq!(deleted.name, "American Express");
        assert_eq!(ports.institutions.institutions.len(), 6);
        assert!(get(&ports.institutions.institutions, InstitutionId::SEED_AMEX).is_none());
    }

    #[test]
    fn an_institution_in_use_is_refused_with_its_accounts_and_nothing_is_written() {
        let mut ports = InMemoryPorts::stub();
        ports.usage.accounts.insert(
            InstitutionId::SEED_AMEX,
            vec!["Amex Platinum".into(), "Amex Gold".into()],
        );
        let before = ports.clone();

        let refused = DeleteInstitution {
            id: InstitutionId::SEED_AMEX,
        }
        .run(&mut ports);

        assert_eq!(
            refused,
            Err(DeleteInstitutionRefusal::InUse(InstitutionUse {
                accounts: vec!["Amex Platinum".into(), "Amex Gold".into()],
            }))
        );
        assert_eq!(ports, before);
    }

    #[test]
    fn a_missing_institution_is_refused() {
        let mut ports = InMemoryPorts::stub();
        let before = ports.clone();

        assert_eq!(
            DeleteInstitution {
                id: InstitutionId::new()
            }
            .run(&mut ports),
            Err(DeleteInstitutionRefusal::NotFound)
        );
        assert_eq!(ports, before);
    }
}
