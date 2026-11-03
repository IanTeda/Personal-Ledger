//! Removes a Document Type, moving its Documents in the same write.

use std::convert::Infallible;

use lib_core::DocumentTypeId;

use super::support::{logged_type, write_documents};
use crate::DocumentType;
use crate::protocols::DocumentRepository;
use crate::types::{RemoveError, fallback_id, id_by_name, remove_type};

/// Removes Document Type `id`. Its Documents move to the type named `destination`, or to the
/// Default when none is named, in the same write, and the destination's Filed count grows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoveDocumentType {
    pub id: DocumentTypeId,
    pub destination: Option<String>,
}

impl RemoveDocumentType {
    pub fn run(self, repo: &mut dyn DocumentRepository) -> Result<(), RemoveError> {
        let outcome = self.apply(repo);
        logged_type("remove document type", self.id, outcome)
    }

    fn apply(&self, repo: &mut dyn DocumentRepository) -> Result<(), RemoveError> {
        let before = repo.document_types();
        let moved_to = self
            .destination
            .as_deref()
            .and_then(|name| id_by_name(&before, name))
            .filter(|moved_to| *moved_to != self.id)
            .unwrap_or_else(|| fallback_id(&before));
        let mut after = before.clone();
        remove_type(&mut after, self.id, self.destination.as_deref())?;
        repo.delete_document_type(self.id);
        for row in after.iter().filter(|row| !before.contains(row)) {
            repo.upsert_document_type(row);
        }
        let Ok(()) = write_documents::<_, Infallible>(repo, |_, documents| {
            for document in documents
                .iter_mut()
                .filter(|document| document.doc_type == DocumentType(self.id))
            {
                document.doc_type = DocumentType(moved_to);
            }
            Ok(())
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactor::test_support::*;
    use crate::interactor::{AddDocumentType, EditDocument};

    #[test]
    fn remove_document_type_moves_its_documents_and_files_in_the_same_write() {
        let mut ports = ports();
        let tax_files = ports.type_row(TAX.0).expect("Tax").files;
        let other_files = ports.type_row(OTHER.0).expect("Other").files;

        RemoveDocumentType {
            id: TAX.0,
            destination: Some("Other".into()),
        }
        .run(&mut ports)
        .expect("Other takes the files");

        assert_eq!(
            ports.writes,
            [
                Write::DeleteType(TAX.0),
                Write::Type(OTHER.0),
                Write::Document(LEASE),
            ]
        );
        assert!(ports.type_row(TAX.0).is_none());
        assert_eq!(
            ports.type_row(OTHER.0).expect("Other").files,
            other_files + tax_files
        );
        assert_eq!(ports.document(LEASE).doc_type, OTHER);
    }

    #[test]
    fn removing_an_empty_type_moves_its_documents_to_the_default() {
        let mut ports = ports();
        let empty = AddDocumentType(draft("Medical")).run(&mut ports);
        EditDocument {
            doc_type: DocumentType(empty),
            ..edit(LEASE, "Lease")
        }
        .run(&mut ports)
        .expect("a Document");
        ports.writes.clear();

        RemoveDocumentType {
            id: empty,
            destination: None,
        }
        .run(&mut ports)
        .expect("an empty type");

        assert_eq!(
            ports.writes,
            [Write::DeleteType(empty), Write::Document(LEASE)]
        );
        assert_eq!(ports.document(LEASE).doc_type, OTHER);
    }

    #[test]
    fn remove_document_type_refuses_the_default_an_unknown_type_and_a_missing_destination() {
        let mut ports = ports();
        let remove = |id: DocumentType, destination: Option<&str>| RemoveDocumentType {
            id: id.0,
            destination: destination.map(str::to_string),
        };

        ports.refuses(RemoveError::IsDefault, |ports| {
            remove(OTHER, None).run(ports)
        });
        ports.refuses(RemoveError::Unknown, |ports| {
            remove(DocumentType(DocumentTypeId::seed(99)), Some("Other")).run(ports)
        });
        ports.refuses(RemoveError::NeedsDestination, |ports| {
            remove(TAX, None).run(ports)
        });
        ports.refuses(RemoveError::NeedsDestination, |ports| {
            remove(TAX, Some("Tax")).run(ports)
        });
    }
}
