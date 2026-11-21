//! Adds a Document Type at the end of the order.

use lib_core::DocumentTypeId;

use super::support::logged_type;
use crate::Result;
use crate::protocols::DocumentRepository;
use crate::types::{DocumentTypeDraft, add_type};

/// Adds a Document Type at the end of the order and returns its id, a fresh one so removing the
/// newest type never frees its id for a saved `type:<id>` filter to match a different type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddDocumentType(pub DocumentTypeDraft);

impl AddDocumentType {
    pub fn run(self, repo: &mut dyn DocumentRepository) -> Result<DocumentTypeId> {
        let id = DocumentTypeId::new();
        let mut types = repo.document_types();
        let outcome = add_type(&mut types, id, &self.0).map(|row| {
            repo.upsert_document_type(row);
            id
        });
        logged_type("add document type", id, outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;
    use crate::interactor::test_support::*;
    use crate::types::{NAME_MAX, NameError};

    #[test]
    fn add_document_type_appends_a_fresh_type_with_no_files() {
        let mut ports = ports();

        let id = AddDocumentType(draft(" Medical "))
            .run(&mut ports)
            .expect("a type");

        assert_eq!(ports.writes, [Write::Type(id)]);
        let added = ports.repo.types.last().expect("a type");
        assert_eq!((added.id, added.name.as_str()), (id, "Medical"));
        assert_eq!((added.files, added.is_default), (0, false));
        assert_ne!(
            AddDocumentType(draft("Dental"))
                .run(&mut ports)
                .expect("a type"),
            id
        );
    }

    #[test]
    fn add_document_type_refuses_an_empty_name() {
        let mut ports = ports();

        ports.refuses(Error::Name(NameError::Empty), |ports| {
            AddDocumentType(draft("  ")).run(ports)
        });
    }

    #[test]
    fn add_document_type_refuses_a_name_over_the_limit() {
        let mut ports = ports();
        let long = "x".repeat(NAME_MAX + 1);

        ports.refuses(Error::Name(NameError::TooLong), |ports| {
            AddDocumentType(draft(&long)).run(ports)
        });
    }

    #[test]
    fn add_document_type_refuses_a_taken_name_in_other_capitals() {
        let mut ports = ports();
        let existing = ports.repo.types[2].name.to_uppercase();

        ports.refuses(Error::Name(NameError::Taken), |ports| {
            AddDocumentType(draft(&existing)).run(ports)
        });
    }
}
