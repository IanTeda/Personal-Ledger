//! Swaps a Document Type one place up or down.

use lib_core::DocumentTypeId;

use crate::protocols::DocumentRepository;
use crate::types::{move_by, position};

/// Swaps Document Type `id` one place down (`delta` 1) or up (`delta` -1), stopping at either
/// end. Returns whether it moved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveDocumentType {
    pub id: DocumentTypeId,
    pub delta: isize,
}

impl MoveDocumentType {
    pub fn run(self, repo: &mut dyn DocumentRepository) -> bool {
        let mut types = repo.document_types();
        let moved = move_by(&mut types, self.id, self.delta);
        if let Some(to) = moved.then(|| position(&types, self.id)).flatten() {
            repo.move_document_type(self.id, to);
        }
        tracing::info!(document_type_id = %self.id, moved, "Moved a Document Type");
        moved
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactor::test_support::*;

    #[test]
    fn move_document_type_swaps_with_its_neighbour_and_stops_at_either_end() {
        let mut ports = ports();
        let first = RECEIPTS.0;

        assert!(
            !MoveDocumentType {
                id: first,
                delta: -1
            }
            .run(&mut ports)
        );
        assert!(ports.writes.is_empty());

        assert!(
            MoveDocumentType {
                id: first,
                delta: 1
            }
            .run(&mut ports)
        );
        assert_eq!(ports.writes, [Write::MoveType(first, 1)]);
        assert_eq!(ports.repo.types[1].id, first);
    }
}
