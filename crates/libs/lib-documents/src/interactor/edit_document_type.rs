//! Applies a draft to a Document Type.

use lib_core::DocumentTypeId;

use super::support::logged_type;
use crate::protocols::DocumentRepository;
use crate::types::{DocumentTypeDraft, UnknownDocumentType, edit_type};

/// Applies `draft` to Document Type `id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditDocumentType {
    pub id: DocumentTypeId,
    pub draft: DocumentTypeDraft,
}

impl EditDocumentType {
    pub fn run(self, repo: &mut dyn DocumentRepository) -> Result<(), UnknownDocumentType> {
        let mut types = repo.document_types();
        let outcome = if edit_type(&mut types, self.id, &self.draft) {
            if let Some(row) = types.iter().find(|row| row.id == self.id) {
                repo.upsert_document_type(row);
            }
            Ok(())
        } else {
            Err(UnknownDocumentType)
        };
        logged_type("edit document type", self.id, outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactor::test_support::*;

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

        ports.refuses(UnknownDocumentType, |ports| {
            EditDocumentType {
                id: DocumentTypeId::seed(99),
                draft: draft("Gone"),
            }
            .run(ports)
        });
    }
}
