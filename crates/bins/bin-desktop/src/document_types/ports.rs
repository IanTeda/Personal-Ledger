//! The Documents store as the Document Types module's Presenters hold it (#966): the store
//! `shell::router::build` is given, shared with the Ledger's forwarding shim and read copy. The
//! writes are committed by the input they were made in (`crate::input::send`), not here.

use std::cell::RefCell;
use std::rc::Rc;

use gpui::{App, Entity};
use lib_core::DocumentTypeId;
use lib_database::SharedDocumentsStore;
use lib_documents::types::DocumentTypeRow;
use lib_documents::{Document, DocumentRepository, SharedRepository};

use crate::ledger_store::LedgerStore;

/// The shared store as the module's Presenters hold the repository, borrowed for each call: the
/// store is shared behind a `RefCell` whose guard can't be lent out of it.
pub struct StoreDocuments(SharedDocumentsStore);

impl DocumentRepository for StoreDocuments {
    fn documents(&self) -> Vec<Document> {
        self.0.borrow().documents().documents()
    }

    fn document_types(&self) -> Vec<DocumentTypeRow> {
        self.0.borrow().documents().document_types()
    }

    fn upsert_document(&mut self, document: &Document) {
        self.0
            .borrow_mut()
            .documents_mut()
            .upsert_document(document);
    }

    fn upsert_document_type(&mut self, row: &DocumentTypeRow) {
        self.0
            .borrow_mut()
            .documents_mut()
            .upsert_document_type(row);
    }

    fn delete_document_type(&mut self, id: DocumentTypeId) {
        self.0.borrow_mut().documents_mut().delete_document_type(id);
    }

    fn move_document_type(&mut self, id: DocumentTypeId, to: usize) {
        self.0
            .borrow_mut()
            .documents_mut()
            .move_document_type(id, to);
    }
}

/// `store` as the Document Types module's Presenters share it.
pub fn shared(store: &SharedDocumentsStore) -> SharedRepository {
    Rc::new(RefCell::new(StoreDocuments(store.clone())))
}

/// Announces `Domain::Documents` and `Domain::DocumentTypes` after a Document Types event or Dialog
/// confirm, so the Documents View redraws what the store now holds: a removal moves Documents, and
/// a reorder or a new type reaches its type filter.
pub fn announce(ledger: &Entity<LedgerStore>, cx: &mut App) {
    ledger.update(cx, |store, cx| store.announce_documents(cx));
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
