//! Accept all strong matches, as one batch.

use std::collections::HashSet;
use std::convert::Infallible;

use lib_core::DocumentId;

use super::support::write_documents;
use crate::protocols::{DocumentReads, DocumentRepository};
use crate::{FiledEntry, accept_all_strong};

/// Accept all strong matches: accepts every Unfiled Document whose Suggested Link is Strong, as
/// one batch, in the Inbox's order with the `skipped` last. Returns what each one wrote; empty
/// when none was Strong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptAllStrong {
    pub skipped: HashSet<DocumentId>,
}

impl AcceptAllStrong {
    pub fn run(
        self,
        reads: &dyn DocumentReads,
        repo: &mut dyn DocumentRepository,
    ) -> Vec<FiledEntry> {
        let transactions = reads.document_transactions();
        let payees = reads.document_payees();
        let Ok(batch) = write_documents::<_, Infallible>(repo, |types, documents| {
            Ok(accept_all_strong(
                types,
                documents,
                &self.skipped,
                &transactions,
                &payees,
            ))
        });
        tracing::info!(accepted = batch.len(), "Accepted every Strong match");
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
            skipped: HashSet::from([WOOLWORTHS_RECEIPT]),
        }
        .run(&records(), &mut ports);

        let filed: Vec<DocumentId> = batch.iter().map(|entry| entry.id).collect();
        assert_eq!(
            filed,
            [COLES_RECEIPT, WOOLWORTHS_RECEIPT],
            "the skipped last"
        );
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
            skipped: HashSet::new(),
        }
        .run(
            &LinkRecords::empty(lib_core::FinancialYearStart::JULY),
            &mut ports,
        );

        assert!(batch.is_empty());
        assert_eq!(ports.repo, before);
        assert!(ports.writes.is_empty());
    }
}
