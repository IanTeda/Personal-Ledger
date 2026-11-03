//! Reverses a filing action whole.

use std::convert::Infallible;

use lib_core::DocumentId;

use super::support::write_documents;
use crate::protocols::DocumentRepository;
use crate::{FilingUndo, undo};

/// Reverses a filing action: each Document it filed returns to Unfiled with its earlier Document
/// Type and date, without the Links the action added. Returns the Document focus goes back to.
#[derive(Debug, Clone, PartialEq)]
pub struct UndoFiling(pub FilingUndo);

impl UndoFiling {
    pub fn run(self, repo: &mut dyn DocumentRepository) -> Option<DocumentId> {
        let documents = self.0.entries.len();
        let Ok(focus) = write_documents::<_, Infallible>(repo, |_, all| Ok(undo(all, self.0)));
        tracing::info!(documents, "Undid a filing");
        focus
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactor::test_support::*;
    use crate::interactor::{Accept, AcceptAllStrong, ToggleLink};
    use crate::{DocumentLink, Filing};
    use lib_core::PayeeId;
    use std::collections::HashSet;

    #[test]
    fn undo_restores_type_date_and_unfiled_and_removes_only_the_links_the_action_added() {
        let mut ports = ports();
        let kept = DocumentLink::Payee(PayeeId::seed(1));
        ToggleLink {
            id: WOOLWORTHS_RECEIPT,
            link: kept,
        }
        .run(&mut ports)
        .expect("an Unfiled Document");
        let before = ports.repo.clone();
        let batch = AcceptAllStrong {
            focus: Some(UNMATCHED),
            skipped: HashSet::new(),
        }
        .run(&records(), &mut ports)
        .expect("two Strong matches");
        ports.writes.clear();

        let focus = UndoFiling(batch).run(&mut ports);

        assert_eq!(focus, Some(UNMATCHED));
        assert_eq!(ports.repo, before);
        assert_eq!(ports.document(WOOLWORTHS_RECEIPT).links, [kept]);
        assert_eq!(
            ports.writes,
            [
                Write::Document(WOOLWORTHS_RECEIPT),
                Write::Document(COLES_RECEIPT),
            ]
        );
    }

    #[test]
    fn undo_keeps_a_link_the_document_held_before_it_was_accepted() {
        let mut ports = ports();
        let link = DocumentLink::Transaction(COLES_SHOP);
        ToggleLink {
            id: COLES_RECEIPT,
            link,
        }
        .run(&mut ports)
        .expect("an Unfiled Document");
        let entry = Accept {
            id: COLES_RECEIPT,
            transaction_id: COLES_SHOP,
        }
        .run(&records(), &mut ports)
        .expect("an Unfiled Document and a Transaction");
        assert!(entry.added_links.is_empty());

        UndoFiling(FilingUndo {
            entries: vec![entry],
            focus: None,
        })
        .run(&mut ports);

        let undone = ports.document(COLES_RECEIPT);
        assert_eq!(undone.filing, Filing::Unfiled);
        assert_eq!(undone.links, [link]);
    }
}
