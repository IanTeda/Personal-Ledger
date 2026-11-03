//! Accept: files an Unfiled Document under a Transaction.

use lib_core::{DocumentId, TransactionId};

use super::support::{logged_document, write_documents};
use crate::protocols::{DocumentRepository, DocumentTransactions};
use crate::{FiledEntry, FilingError, accept};

/// Accept: files Unfiled Document `id` under Transaction `transaction_id`, with the extracted
/// Document Type and date. Returns what [`UndoFiling`] needs to reverse it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Accept {
    pub id: DocumentId,
    pub transaction_id: TransactionId,
}

impl Accept {
    pub fn run(
        self,
        transactions: &dyn DocumentTransactions,
        repo: &mut dyn DocumentRepository,
    ) -> Result<FiledEntry, FilingError> {
        let transactions = transactions.document_transactions();
        logged_document(
            "accept",
            self.id,
            write_documents(repo, |types, documents| {
                accept(
                    types,
                    documents,
                    &transactions,
                    self.id,
                    self.transaction_id,
                )
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactor::test_support::*;
    use crate::{DocumentLink, Filing};

    #[test]
    fn accept_files_under_the_transaction_with_the_extracted_type_and_date() {
        let mut ports = ports();

        let entry = Accept {
            id: COLES_RECEIPT,
            transaction_id: COLES_SHOP,
        }
        .run(&records(), &mut ports)
        .expect("an Unfiled Document and a Transaction");

        assert_eq!(ports.writes, [Write::Document(COLES_RECEIPT)]);
        let filed = ports.document(COLES_RECEIPT);
        assert_eq!(filed.filing, Filing::Filed);
        assert_eq!(filed.doc_type, RECEIPTS);
        assert_eq!(filed.date, today() - chrono::Duration::days(1));
        assert_eq!(filed.links, [DocumentLink::Transaction(COLES_SHOP)]);
        assert_eq!(entry.added_links, filed.links);
        assert_eq!(entry.previous_type, OTHER);
    }

    #[test]
    fn accept_refuses_an_unknown_transaction_a_filed_and_an_unknown_document() {
        let mut ports = ports();
        let accept = |id, transaction_id| Accept { id, transaction_id };

        ports.refuses(FilingError::NoSuchTransaction, |ports| {
            accept(COLES_RECEIPT, TransactionId::seed(99)).run(&records(), ports)
        });
        ports.refuses(FilingError::AlreadyFiled, |ports| {
            accept(LEASE, COLES_SHOP).run(&records(), ports)
        });
        ports.refuses(FilingError::NotFound, |ports| {
            accept(DocumentId::seed(99), COLES_SHOP).run(&records(), ports)
        });
    }
}
