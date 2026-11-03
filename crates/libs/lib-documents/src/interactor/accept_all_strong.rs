//! Accept all strong matches, as one undoable batch.

use std::collections::HashSet;
use std::convert::Infallible;

use lib_core::DocumentId;

use super::support::write_documents;
use crate::protocols::{DocumentReads, DocumentRepository};
use crate::{FilingUndo, accept_all_strong};

/// Accept all strong matches: accepts every Unfiled Document whose Suggested Link is Strong, as
/// one undoable batch, in the Inbox's order with the `skipped` last. `focus` is the Inbox row the
/// Client keeps in view. Returns `None` when none was Strong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptAllStrong {
    pub focus: Option<DocumentId>,
    pub skipped: HashSet<DocumentId>,
}

impl AcceptAllStrong {
    pub fn run(
        self,
        reads: &dyn DocumentReads,
        repo: &mut dyn DocumentRepository,
    ) -> Option<FilingUndo> {
        let transactions = reads.document_transactions();
        let payees = reads.document_payees();
        let Ok(batch) = write_documents::<_, Infallible>(repo, |types, documents| {
            Ok(accept_all_strong(
                types,
                documents,
                &self.skipped,
                &transactions,
                &payees,
                self.focus,
            ))
        });
        let accepted = batch.as_ref().map_or(0, |batch| batch.entries.len());
        tracing::info!(accepted, "Accepted every Strong match");
        batch
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactor::test_support::*;
    use crate::{DocumentLink, Filing, LinkRecords};

    #[test]
    fn accept_all_strong_writes_every_strong_match_in_one_batch() {
        let mut ports = ports();

        let batch = AcceptAllStrong {
            focus: Some(UNMATCHED),
            skipped: HashSet::from([WOOLWORTHS_RECEIPT]),
        }
        .run(&records(), &mut ports)
        .expect("two Strong matches");

        let filed: Vec<DocumentId> = batch.entries.iter().map(|entry| entry.id).collect();
        assert_eq!(
            filed,
            [COLES_RECEIPT, WOOLWORTHS_RECEIPT],
            "the skipped last"
        );
        assert_eq!(batch.focus, Some(UNMATCHED));
        assert_eq!(
            ports.writes,
            [
                Write::Document(WOOLWORTHS_RECEIPT),
                Write::Document(COLES_RECEIPT),
            ]
        );
        assert_eq!(ports.document(UNMATCHED).filing, Filing::Unfiled);
        assert_eq!(
            ports.document(WOOLWORTHS_RECEIPT).links,
            [DocumentLink::Transaction(WOOLWORTHS_SHOP)]
        );
    }

    #[test]
    fn accept_all_strong_with_no_strong_match_writes_nothing() {
        let mut ports = ports();
        let before = ports.repo.clone();

        let batch = AcceptAllStrong {
            focus: None,
            skipped: HashSet::new(),
        }
        .run(&LinkRecords::default(), &mut ports);

        assert_eq!(batch, None);
        assert_eq!(ports.repo, before);
        assert!(ports.writes.is_empty());
    }
}
