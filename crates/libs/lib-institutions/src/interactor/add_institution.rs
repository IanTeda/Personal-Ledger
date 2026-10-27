use crate::protocols::InstitutionRepository;
use crate::{Institution, InstitutionDraft, InstitutionError, insert_institution};

/// Add an Institution, its name trimmed and refused when blank or already taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddInstitution(pub InstitutionDraft);

impl AddInstitution {
    /// Returns the Institution as written.
    pub fn run(
        self,
        institutions: &mut dyn InstitutionRepository,
    ) -> Result<Institution, InstitutionError> {
        let outcome = self.apply(institutions);
        match &outcome {
            Ok(institution) => {
                tracing::info!(institution_id = %institution.id, "Added an Institution")
            }
            Err(refusal) => tracing::info!(%refusal, "Refused to add an Institution"),
        }
        outcome
    }

    fn apply(
        self,
        institutions: &mut dyn InstitutionRepository,
    ) -> Result<Institution, InstitutionError> {
        let mut rows = institutions.institutions();
        let institution = insert_institution(&mut rows, &self.0)?;
        institutions.add_institution(institution.clone());
        Ok(institution)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::InMemoryInstitutions;

    fn draft(name: &str) -> InstitutionDraft {
        InstitutionDraft {
            name: name.to_string(),
            products: "loan".to_string(),
        }
    }

    #[test]
    fn adding_an_institution_writes_it_trimmed_after_the_others() {
        let mut institutions = InMemoryInstitutions::stub();

        let added = AddInstitution(draft(" Bank X "))
            .run(&mut institutions)
            .expect("a free name");

        assert_eq!(added.name, "Bank X");
        assert_eq!(added.products, "loan");
        assert_eq!(institutions.institutions().last(), Some(&added));
        assert_eq!(institutions.institution(added.id), Some(added));
    }

    #[test]
    fn a_blank_or_taken_name_is_refused_and_nothing_is_written() {
        let mut institutions = InMemoryInstitutions::stub();
        let before = institutions.clone();

        assert_eq!(
            AddInstitution(draft("westpac banking")).run(&mut institutions),
            Err(InstitutionError::DuplicateName("Westpac Banking".into()))
        );
        assert_eq!(
            AddInstitution(draft(" ")).run(&mut institutions),
            Err(InstitutionError::NameRequired)
        );
        assert_eq!(institutions, before);
    }
}
