//! Deletes a Document, Filed or Unfiled, keeping its file and the records it links.

use lib_core::DocumentId;

use super::support::logged_document;
use crate::protocols::DocumentRepository;
use crate::{Document, get};

/// Why a delete wrote nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteDocumentError {
    /// No Document has that id: it has gone since the Dialog opened.
    NotFound,
}

/// Deletes Document `id` and its Document Links in one write. The file on disk and every record
/// the Links named stay, since the Ledger holds only a reference. Returns the Document as it was,
/// for the Toast that names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeleteDocument(pub DocumentId);

impl DeleteDocument {
    pub fn run(self, repo: &mut dyn DocumentRepository) -> Result<Document, DeleteDocumentError> {
        let outcome = get(&repo.documents(), self.0)
            .cloned()
            .ok_or(DeleteDocumentError::NotFound)
            .inspect(|_| repo.delete_document(self.0));
        logged_document("delete document", self.0, outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactor::test_support::*;

    #[test]
    fn delete_removes_a_filed_and_an_unfiled_document_in_one_write_each() {
        let mut ports = ports();

        let lease = DeleteDocument(LEASE).run(&mut ports).expect("a Filed one");
        let receipt = DeleteDocument(COLES_RECEIPT)
            .run(&mut ports)
            .expect("an Unfiled one");

        assert_eq!(lease.id, LEASE);
        assert_eq!(receipt.id, COLES_RECEIPT);
        assert_eq!(
            ports.writes,
            [
                Write::DeleteDocument(LEASE),
                Write::DeleteDocument(COLES_RECEIPT)
            ]
        );
        assert!(get(&ports.repo.documents, LEASE).is_none());
        assert_eq!(ports.repo.documents.len(), 2);
    }

    #[test]
    fn delete_refuses_a_document_that_has_gone_and_writes_nothing() {
        let mut ports = ports();

        ports.refuses(DeleteDocumentError::NotFound, |ports| {
            DeleteDocument(DocumentId::seed(99)).run(ports)
        });
    }

    #[test]
    fn deleting_twice_refuses_the_second() {
        let mut ports = ports();
        DeleteDocument(LEASE).run(&mut ports).expect("there");

        assert_eq!(
            DeleteDocument(LEASE).run(&mut ports),
            Err(DeleteDocumentError::NotFound)
        );
    }
}
