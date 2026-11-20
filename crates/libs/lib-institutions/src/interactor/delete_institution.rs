use lib_core::InstitutionId;

use crate::protocols::InstitutionsPorts;
use crate::{Error, Institution, Result};

/// Delete an Institution no Account names. Usage is read here, so the refusal is decided in one
/// place whatever the caller checked when it opened its Dialog: an Account may have come to name
/// the Institution since.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteInstitution {
    pub id: InstitutionId,
}

impl DeleteInstitution {
    /// Returns the Institution removed.
    pub fn run(self, ports: &mut dyn InstitutionsPorts) -> Result<Institution> {
        let outcome = self.apply(ports);
        match &outcome {
            Ok(_) => tracing::info!(institution_id = %self.id, "Deleted an Institution"),
            Err(refusal) => {
                tracing::info!(institution_id = %self.id, %refusal, "Refused to delete an Institution");
            }
        }
        outcome
    }

    fn apply(self, ports: &mut dyn InstitutionsPorts) -> Result<Institution> {
        let institution = ports
            .institutions()
            .institution(self.id)
            .ok_or(Error::NotFound)?;
        let uses = ports.usage().institution_usage(self.id);
        if uses.is_used() {
            return Err(Error::InUse(uses));
        }
        ports.institutions_mut().delete_institution(self.id);
        Ok(institution)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InstitutionUse;
    use crate::fake::InMemoryPorts;
    use crate::get;

    #[test]
    fn an_unused_institution_is_removed_and_returned() {
        let mut ports = InMemoryPorts::sample();

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
        let mut ports = InMemoryPorts::sample();
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
            Err(Error::InUse(InstitutionUse {
                accounts: vec!["Amex Platinum".into(), "Amex Gold".into()],
            }))
        );
        assert_eq!(ports, before);
    }

    #[test]
    fn a_missing_institution_is_refused() {
        let mut ports = InMemoryPorts::sample();
        let before = ports.clone();

        assert_eq!(
            DeleteInstitution {
                id: InstitutionId::new()
            }
            .run(&mut ports),
            Err(Error::NotFound)
        );
        assert_eq!(ports, before);
    }
}
