//! The Documents store as the Document Types module's Presenters hold it (#966): the store
//! `shell::router::build` is given, shared with the Documents page. The
//! writes are committed by the input they were made in (`crate::input::send`), not here.

use std::cell::{Cell, RefCell, RefMut};
use std::rc::Rc;

use gpui::{App, Entity};
use lib_core::{DocumentId, DocumentTypeId};
use lib_database::{DocumentsStore, SharedDocumentsStore};
use lib_documents::types::DocumentTypeRow;
use lib_documents::{Document, DocumentRepository, SharedRepository};

use crate::ledger_store::LedgerStore;

/// Whether a write went through a [`StoreDocuments`] since the last ask, so the Documents page
/// announces the Documents only after an event that wrote them. A clone shares the one flag.
#[derive(Clone, Default)]
pub struct Written(Rc<Cell<bool>>);

impl Written {
    /// Whether anything was written since the last ask.
    pub fn take(&self) -> bool {
        self.0.take()
    }
}

/// The shared store as the module's Presenters hold the repository, borrowed for each call: the
/// store is shared behind a `RefCell` whose guard can't be lent out of it. Each write sets
/// `written`.
pub struct StoreDocuments {
    store: SharedDocumentsStore,
    written: Written,
}

impl StoreDocuments {
    /// The store to write, noting the write.
    fn write(&self) -> RefMut<'_, dyn DocumentsStore> {
        self.written.0.set(true);
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

/// `store` as the Document Types module's Presenters and the Documents page share it, each write
/// noted in `written`.
pub fn shared(store: &SharedDocumentsStore, written: &Written) -> SharedRepository {
    Rc::new(RefCell::new(StoreDocuments {
        store: store.clone(),
        written: written.clone(),
    }))
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
        let written = Written::default();
        let shared = shared(&store, &written);
        assert!(
            !written.take(),
            "nothing is written before an Interactor runs"
        );

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
        assert!(written.take(), "the writes are noted");
        assert!(!written.take(), "and the note is taken once");
    }
}
