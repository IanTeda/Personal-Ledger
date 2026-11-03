//! Adds a Document Type at the end of the order.

use lib_core::DocumentTypeId;

use crate::protocols::DocumentRepository;
use crate::types::{DocumentTypeDraft, add_type};

/// Adds a Document Type at the end of the order and returns its id, a fresh one so removing the
/// newest type never frees its id for a saved `type:<id>` filter to match a different type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddDocumentType(pub DocumentTypeDraft);

impl AddDocumentType {
    pub fn run(self, repo: &mut dyn DocumentRepository) -> DocumentTypeId {
        let id = DocumentTypeId::new();
        let mut types = repo.document_types();
        add_type(&mut types, id, &self.0);
        if let Some(row) = types.last() {
            repo.upsert_document_type(row);
        }
        tracing::info!(document_type_id = %id, "Added a Document Type");
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactor::test_support::*;

    #[test]
    fn add_document_type_appends_a_fresh_type_with_no_files() {
        let mut ports = ports();

        let id = AddDocumentType(draft(" Medical ")).run(&mut ports);

        assert_eq!(ports.writes, [Write::Type(id)]);
        let added = ports.repo.types.last().expect("a type");
        assert_eq!((added.id, added.name.as_str()), (id, "Medical"));
        assert_eq!((added.files, added.is_default), (0, false));
        assert_ne!(AddDocumentType(draft("Medical")).run(&mut ports), id);
    }
}
