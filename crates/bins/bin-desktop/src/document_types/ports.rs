//! The Documents store as the Document Types module's Presenters hold it (#966): the store
//! `shell::router::build` is given, shared with the Documents page. The
//! writes are committed and announced by the input they were made in (`crate::input::send`), not
//! here.

use std::cell::RefCell;
use std::rc::Rc;

use lib_core::{DocumentId, DocumentTypeId};
use lib_database::SharedDocumentsStore;
use lib_documents::types::DocumentTypeRow;
use lib_documents::{Document, DocumentRepository, SharedRepository};

/// The shared store as the module's Presenters hold the repository, borrowed for each call: the
/// store is shared behind a `RefCell` whose guard can't be lent out of it.
pub struct StoreDocuments {
    store: SharedDocumentsStore,
}

impl StoreDocuments {
    fn write(&self) -> std::cell::RefMut<'_, dyn lib_database::DocumentsStore> {
        self.store.borrow_mut()
    }
}

impl DocumentRepository for StoreDocuments {
    fn documents(&self) -> Vec<Document> {
        self.store.borrow().documents().documents()
    }

    fn document_types(&self) -> Vec<DocumentTypeRow> {
        self.store.borrow().documents().document_types()
    }

    fn upsert_document(&mut self, document: &Document) {
        self.write().documents_mut().upsert_document(document);
    }

    fn delete_document(&mut self, id: DocumentId) {
        self.write().documents_mut().delete_document(id);
    }

    fn upsert_document_type(&mut self, row: &DocumentTypeRow) {
        self.write().documents_mut().upsert_document_type(row);
    }

    fn delete_document_type(&mut self, id: DocumentTypeId) {
        self.write().documents_mut().delete_document_type(id);
    }

    fn move_document_type(&mut self, id: DocumentTypeId, to: usize) {
        self.write().documents_mut().move_document_type(id, to);
    }
}

/// `store` as the Document Types module's Presenters and the Documents page share it.
pub fn shared(store: &SharedDocumentsStore) -> SharedRepository {
    Rc::new(RefCell::new(StoreDocuments {
        store: store.clone(),
    }))
}

#[cfg(test)]
mod tests {
    use lib_documents::fake::InMemoryDocuments;
    use lib_documents::interactor::{MoveDocumentType, RemoveDocumentType};
    use lib_documents::types::default_types;

    use super::*;

    fn store() -> SharedDocumentsStore {
        Rc::new(RefCell::new(InMemoryDocuments {
            documents: Vec::new(),
            types: default_types(),
        }))
    }

    #[test]
    fn the_interactors_write_the_shared_store_through_the_wrapper() {
        let store = store();
        let shared = shared(&store);

        assert!(
            MoveDocumentType {
                id: DocumentTypeId::seed(1),
                delta: 1,
            }
            .run(&mut *shared.borrow_mut())
        );
        RemoveDocumentType {
            id: DocumentTypeId::seed(3),
            destination: Some("Other".into()),
        }
        .run(&mut *shared.borrow_mut())
        .expect("a removable type");

        let stored = store.borrow().documents().document_types();
        assert_eq!(stored.len(), 8);
        assert_eq!(stored[1].name, "Receipts");
        assert_eq!(stored.last().map(|row| row.files), Some(33));
        assert_eq!(shared.borrow().document_types(), stored);
    }
}
