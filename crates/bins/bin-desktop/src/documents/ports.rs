//! The Documents store as the Desktop holds it (#963): the store `shell::router::build` is given,
//! shared by the Ledger's forwarding shim and its read copy. The writes are committed by the input
//! they were made in (`crate::input::send`), not here.
//!
//! Every Documents writer is still a `LedgerCommand`, so [`forward`] carries what each wrote to
//! the store, and the Ledger keeps a read copy for the Documents View, refreshed after each write
//! and on rollback, until it runs on `lib-documents`' Presenters (#967, #968). The Document Types
//! page writes the store itself, through the Interactors (#966).

use std::cell::RefCell;
use std::rc::Rc;

use lib_database::{DocumentsStore, SharedDocumentsStore};
use lib_documents::fake::InMemoryDocuments;
use lib_ledger::{Ledger, LedgerChange};

/// The Ledger's Documents and Document Types as an in-memory store, for a test that builds the
/// Ledger without storage.
pub fn in_memory(ledger: &Ledger) -> SharedDocumentsStore {
    Rc::new(RefCell::new(InMemoryDocuments {
        documents: ledger.documents().to_vec(),
        types: ledger.document_types().to_vec(),
    }))
}

/// The forwarding shim (#963): sends each Document upsert in `changes` to `store`, in the order
/// recorded, returning whether it wrote anything. The Document Types are written by their own
/// module (#966), so what is left is the Documents View's writers, until it moves (#967).
pub fn forward(store: &mut dyn DocumentsStore, changes: &[LedgerChange]) -> bool {
    let port = store.documents_mut();
    let mut wrote = false;
    for change in changes {
        if let LedgerChange::UpsertDocument(document) = change {
            port.upsert_document(document);
            wrote = true;
        }
    }
    wrote
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use lib_core::DocumentId;
    use lib_ledger::{AddDocument, LedgerCommand};

    use super::*;

    fn ledger() -> Ledger {
        Ledger::stub_on(NaiveDate::from_ymd_opt(2026, 10, 2).expect("a real date"))
    }

    #[test]
    fn an_added_document_reaches_the_store_and_nothing_else_does() {
        let mut ledger = ledger();
        let store = in_memory(&ledger);
        let added = lib_documents::NewDocument {
            path: std::path::PathBuf::from("/tmp/lease.pdf"),
            bytes: 1,
            title: "Lease".to_string(),
            doc_type: lib_documents::DocumentType::TAX,
            date: NaiveDate::from_ymd_opt(2026, 10, 2).expect("a real date"),
            key_date: None,
        };
        let id: DocumentId = AddDocument(added).run(&mut ledger).expect("a new Document");
        let changes = ledger.drain_changes();

        assert!(forward(&mut *store.borrow_mut(), &changes));

        assert!(
            store
                .borrow()
                .documents()
                .documents()
                .iter()
                .any(|document| document.id == id)
        );
        assert!(!forward(
            &mut *store.borrow_mut(),
            &[LedgerChange::DeleteTag(lib_core::TagId::seed(1))]
        ));
    }

    #[test]
    fn nothing_of_documents_writes_nothing() {
        let store = in_memory(&ledger());
        assert!(!forward(&mut *store.borrow_mut(), &[]));
    }
}
