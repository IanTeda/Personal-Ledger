//! Applies a draft to a Document Type.

use lib_core::DocumentTypeId;

use super::support::logged_type;
use crate::Result;
use crate::protocols::DocumentRepository;
use crate::types::{DocumentTypeDraft, edit_type};

/// Applies `draft` to Document Type `id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditDocumentType {
    pub id: DocumentTypeId,
    pub draft: DocumentTypeDraft,
}

impl EditDocumentType {
    pub fn run(self, repo: &mut dyn DocumentRepository) -> Result<()> {
        let mut types = repo.document_types();
        let outcome =
            edit_type(&mut types, self.id, &self.draft).map(|row| repo.upsert_document_type(row));
        logged_type("edit document type", self.id, outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;
    use crate::interactor::test_support::*;
    use crate::types::NameError;

    #[test]
    fn edit_document_type_rewrites_the_type_in_place() {
        let mut ports = ports();

        EditDocumentType {
            id: TAX.0,
            draft: draft("Taxation"),
        }
        .run(&mut ports)
        .expect("a type");

        assert_eq!(ports.writes, [Write::Type(TAX.0)]);
        assert_eq!(ports.repo.types[2].name, "Taxation");
    }

    #[test]
    fn edit_document_type_refuses_an_unknown_type() {
        let mut ports = ports();

        ports.refuses(Error::UnknownDocumentType, |ports| {
            EditDocumentType {
                id: DocumentTypeId::seed(99),
                draft: draft("Gone"),
            }
            .run(ports)
        });
    }

    #[test]
    fn edit_document_type_refuses_another_types_name() {
        let mut ports = ports();
        let other = ports.repo.types[0].name.to_uppercase();

        ports.refuses(Error::Name(NameError::Taken), |ports| {
            EditDocumentType {
                id: TAX.0,
                draft: draft(&other),
            }
            .run(ports)
        });
    }

    #[test]
    fn edit_document_type_accepts_its_own_name_in_other_capitals() {
        let mut ports = ports();
        let own = ports.repo.types[2].name.to_uppercase();

        EditDocumentType {
            id: TAX.0,
            draft: draft(&own),
        }
        .run(&mut ports)
        .expect("its own name");

        assert_eq!(ports.writes, [Write::Type(TAX.0)]);
    }
}
