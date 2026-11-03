//! File… / Link elsewhere…: files an Unfiled Document by hand.

use lib_core::DocumentId;

use super::support::{logged_document, write_documents};
use crate::protocols::DocumentRepository;
use crate::{DocumentLink, DocumentType, FiledEntry, FilingError, file_by_hand};

/// File… / Link elsewhere…: files Unfiled Document `id` by hand with a Document Type and at most
/// one Link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileByHand {
    pub id: DocumentId,
    pub doc_type: DocumentType,
    pub link: Option<DocumentLink>,
}

impl FileByHand {
    pub fn run(self, repo: &mut dyn DocumentRepository) -> Result<FiledEntry, FilingError> {
        logged_document(
            "file by hand",
            self.id,
            write_documents(repo, |_, documents| {
                file_by_hand(documents, self.id, self.doc_type, self.link)
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Filing;
    use crate::interactor::test_support::*;
    use lib_core::AccountId;

    #[test]
    fn file_by_hand_files_with_the_chosen_type_and_link() {
        let mut ports = ports();
        let link = DocumentLink::Account(AccountId::seed(1));

        FileByHand {
            id: UNMATCHED,
            doc_type: TAX,
            link: Some(link),
        }
        .run(&mut ports)
        .expect("an Unfiled Document");

        assert_eq!(ports.writes, [Write::Document(UNMATCHED)]);
        let filed = ports.document(UNMATCHED);
        assert_eq!((filed.filing, filed.doc_type), (Filing::Filed, TAX));
        assert_eq!(filed.links, [link]);
    }

    #[test]
    fn file_by_hand_refuses_a_filed_and_an_unknown_document() {
        let mut ports = ports();
        let file = |id| FileByHand {
            id,
            doc_type: TAX,
            link: None,
        };

        ports.refuses(FilingError::AlreadyFiled, |ports| file(LEASE).run(ports));
        ports.refuses(FilingError::NotFound, |ports| {
            file(DocumentId::seed(99)).run(ports)
        });
    }
}
