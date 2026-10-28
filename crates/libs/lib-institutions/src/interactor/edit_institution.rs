use lib_core::InstitutionId;

use crate::protocols::InstitutionRepository;
use crate::{Institution, InstitutionDraft, InstitutionError, edit_institution, get};

/// Edit an Institution's name and Products. The name is refused when blank or held by another
/// Institution; a change of capitals only is accepted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditInstitution {
    pub id: InstitutionId,
    pub draft: InstitutionDraft,
}

impl EditInstitution {
    /// Returns the Institution as written.
    pub fn run(
        self,
        institutions: &mut dyn InstitutionRepository,
    ) -> Result<Institution, InstitutionError> {
        let id = self.id;
        let outcome = self.apply(institutions);
        match &outcome {
            Ok(_) => tracing::info!(institution_id = %id, "Edited an Institution"),
            Err(refusal) => {
                tracing::info!(institution_id = %id, %refusal, "Refused to edit an Institution")
            }
        }
        outcome
    }

    fn apply(
        self,
        institutions: &mut dyn InstitutionRepository,
    ) -> Result<Institution, InstitutionError> {
        let mut rows = institutions.institutions();
        edit_institution(&mut rows, self.id, &self.draft)?;
        let institution = get(&rows, self.id)
            .cloned()
            .ok_or(InstitutionError::NotFound)?;
        institutions.upsert_institution(institution.clone());
        Ok(institution)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::InMemoryInstitutions;

    fn draft(name: &str, products: &str) -> InstitutionDraft {
        InstitutionDraft {
            name: name.to_string(),
            products: products.to_string(),
            usual_unit: None,
        }
    }

    #[test]
    fn an_edit_writes_the_row_where_it_was() {
        let mut institutions = InMemoryInstitutions::stub();

        let edited = EditInstitution {
            id: InstitutionId::SEED_AMEX,
            draft: draft(" Amex ", "credit card \u{b7} cash"),
        }
        .run(&mut institutions)
        .expect("a free name");

        assert_eq!(edited.name, "Amex");
        assert_eq!(institutions.institutions[2], edited);
        assert_eq!(institutions.institutions.len(), 7);
    }

    #[test]
    fn a_refused_edit_writes_nothing() {
        let mut institutions = InMemoryInstitutions::stub();
        let before = institutions.clone();

        let refused = EditInstitution {
            id: InstitutionId::SEED_AMEX,
            draft: draft("anz banking group", "loan"),
        }
        .run(&mut institutions);

        assert_eq!(
            refused,
            Err(InstitutionError::DuplicateName("ANZ Banking Group".into()))
        );
        assert_eq!(institutions, before);
    }
}
