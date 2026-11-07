//! `lib-documents`' repository answered over the Documents store, for the Documents page and the
//! Document Types page alike (#966, #1083). The read ports a Link names are
//! [`StoredDocumentReads`](crate::StoredDocumentReads).

use lib_core::{DocumentId, DocumentTypeId};
use lib_documents::types::DocumentTypeRow;
use lib_documents::{Document, DocumentRepository};

use crate::adapter::store::SharedDocumentsStore;

/// [`DocumentRepository`] over the shared Documents store, borrowed for each call: the store is
/// shared behind a `RefCell` whose guard can't be lent out of it. A write reaches the store's
/// recorder, which names the Documents or the Document Types by the write, so the Document Types
/// page hears only its own.
pub struct StoredDocumentsPorts {
    store: SharedDocumentsStore,
}

impl StoredDocumentsPorts {
    pub fn new(store: SharedDocumentsStore) -> Self {
        Self { store }
    }
}

impl DocumentRepository for StoredDocumentsPorts {
    fn documents(&self) -> Vec<Document> {
        self.store.borrow().documents().documents()
    }

    fn document_types(&self) -> Vec<DocumentTypeRow> {
        self.store.borrow().documents().document_types()
    }

    fn upsert_document(&mut self, document: &Document) {
        self.store
            .borrow_mut()
            .documents_mut()
            .upsert_document(document);
    }

    fn delete_document(&mut self, id: DocumentId) {
        self.store.borrow_mut().documents_mut().delete_document(id);
    }

    fn upsert_document_type(&mut self, row: &DocumentTypeRow) {
        self.store
            .borrow_mut()
            .documents_mut()
            .upsert_document_type(row);
    }

    fn delete_document_type(&mut self, id: DocumentTypeId) {
        self.store
            .borrow_mut()
            .documents_mut()
            .delete_document_type(id);
    }

    fn move_document_type(&mut self, id: DocumentTypeId, to: usize) {
        self.store
            .borrow_mut()
            .documents_mut()
            .move_document_type(id, to);
    }
}

#[cfg(test)]
mod tests {
    use crate::ports::test_support::{fakes, file};
    use crate::{Domain, LedgerPorts};

    #[test]
    fn a_document_type_write_reports_the_document_types_written_on_either() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        for ledger in [fakes(), file(dir.path())] {
            let LedgerPorts {
                documents, commit, ..
            } = ledger;
            let mut row = documents
                .borrow()
                .document_types()
                .into_iter()
                .next()
                .expect("a default Document Type");
            assert!(commit.commit().is_empty(), "a read writes nothing");
            row.name.push_str(" renamed");

            documents.borrow_mut().upsert_document_type(&row);

            assert_eq!(commit.commit().wrote, [Domain::DocumentTypes]);
        }
    }

    #[test]
    fn a_document_write_reports_the_documents_written_on_either() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        for ledger in [fakes(), file(dir.path())] {
            let LedgerPorts {
                documents, commit, ..
            } = ledger;
            let document = documents
                .borrow()
                .documents()
                .into_iter()
                .next()
                .expect("a seeded Document");

            documents.borrow_mut().upsert_document(&document);

            assert_eq!(commit.commit().wrote, [Domain::Documents]);
        }
    }
}
