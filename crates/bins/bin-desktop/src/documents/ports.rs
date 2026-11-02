//! The Documents store as the Desktop holds it (#963): the store `shell::router::build` is given,
//! shared by the Ledger's forwarding shim and its read copy. The writes are committed by the input
//! they were made in (`crate::input::send`), not here.
//!
//! Every Documents and Document Types writer is still a `LedgerCommand`, so [`forward`] carries
//! what each wrote to the store, and the Ledger keeps a read copy for the Documents View and
//! Settings › Documents, refreshed after each write and on rollback, until they run on
//! `lib-documents`' Presenters (#966 to #968).

use std::cell::RefCell;
use std::rc::Rc;

use lib_database::{DocumentsStore, SharedDocumentsStore};
use lib_documents::fake::InMemoryDocuments;
use lib_documents::types::DocumentTypeRow;
use lib_ledger::{Ledger, LedgerChange};

/// The Ledger's Documents and Document Types as an in-memory store, for a test that builds the
/// Ledger without storage.
pub fn in_memory(ledger: &Ledger) -> SharedDocumentsStore {
    Rc::new(RefCell::new(InMemoryDocuments {
        documents: ledger.documents().to_vec(),
        types: ledger.document_types().to_vec(),
    }))
}

/// The forwarding shim (#963): sends each Document and Document Type upsert and each Document Type
/// delete in `changes` to `store`, in the order recorded, returning whether it wrote anything. A
/// change carries a type's row but not its place, so each written type is then moved to where it
/// stands in `order`, the Ledger's list after the command.
pub fn forward(
    store: &mut dyn DocumentsStore,
    changes: &[LedgerChange],
    order: &[DocumentTypeRow],
) -> bool {
    let port = store.documents_mut();
    let mut wrote = false;
    for change in changes {
        match change {
            LedgerChange::UpsertDocument(document) => port.upsert_document(document),
            LedgerChange::UpsertDocumentType(row) => port.upsert_document_type(row),
            LedgerChange::DeleteDocumentType(id) => port.delete_document_type(*id),
            _ => continue,
        }
        wrote = true;
    }
    for change in changes {
        let LedgerChange::UpsertDocumentType(row) = change else {
            continue;
        };
        let Some(to) = order.iter().position(|held| held.id == row.id) else {
            continue;
        };
        let stored = port.document_types();
        if stored.get(to).map(|held| held.id) != Some(row.id) {
            port.move_document_type(row.id, to);
        }
    }
    wrote
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use lib_core::DocumentTypeId;
    use lib_ledger::{LedgerCommand, MoveDocumentType, RemoveDocumentType};

    use super::*;

    fn ledger() -> Ledger {
        Ledger::stub_on(NaiveDate::from_ymd_opt(2026, 10, 2).expect("a real date"))
    }

    /// Runs `command` on `ledger` and forwards what it wrote to `store`, as `LedgerStore::apply`
    /// does.
    fn apply<C: LedgerCommand>(ledger: &mut Ledger, store: &SharedDocumentsStore, command: C) {
        assert!(command.run(ledger).is_ok(), "the command lands");
        let changes = ledger.drain_changes();
        assert!(forward(
            &mut *store.borrow_mut(),
            &changes,
            ledger.document_types()
        ));
    }

    #[test]
    fn a_moved_type_takes_its_place_in_the_store() {
        let mut ledger = ledger();
        let store = in_memory(&ledger);

        apply(
            &mut ledger,
            &store,
            MoveDocumentType {
                id: DocumentTypeId::seed(1),
                delta: 1,
            },
        );

        assert_eq!(
            store.borrow().documents().document_types(),
            ledger.document_types()
        );
    }

    #[test]
    fn a_removed_type_leaves_the_store_with_its_documents_moved() {
        let mut ledger = ledger();
        let store = in_memory(&ledger);

        apply(
            &mut ledger,
            &store,
            RemoveDocumentType {
                id: DocumentTypeId::seed(3),
                destination: Some("Other".into()),
            },
        );

        let stored = store.borrow();
        assert_eq!(stored.documents().document_types(), ledger.document_types());
        assert_eq!(stored.documents().documents(), ledger.documents());
    }

    #[test]
    fn nothing_of_documents_writes_nothing() {
        let store = in_memory(&ledger());
        assert!(!forward(&mut *store.borrow_mut(), &[], &[]));
    }
}
