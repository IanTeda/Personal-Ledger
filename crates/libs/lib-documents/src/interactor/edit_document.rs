//! The Library's `e`: replaces a Filed Document's Title, Document Type, date and Key Date.

use chrono::NaiveDate;
use lib_core::DocumentId;

use super::support::{logged_document, write_documents};
use crate::protocols::DocumentRepository;
use crate::{AddError, DocumentType, KeyDate, edit_metadata};

/// The Library's `e`: replaces Filed Document `id`'s Title, Document Type, date and Key Date.
#[derive(Debug, Clone, PartialEq)]
pub struct EditDocument {
    pub id: DocumentId,
    pub title: String,
    pub doc_type: DocumentType,
    pub date: NaiveDate,
    pub key_date: Option<KeyDate>,
}

impl EditDocument {
    pub fn run(self, repo: &mut dyn DocumentRepository) -> Result<(), AddError> {
        logged_document(
            "edit document",
            self.id,
            write_documents(repo, |_, documents| {
                edit_metadata(
                    documents,
                    self.id,
                    &self.title,
                    self.doc_type,
                    self.date,
                    self.key_date,
                )
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactor::test_support::*;

    #[test]
    fn edit_rewrites_the_documents_metadata() {
        let mut ports = ports();

        edit(LEASE, " Lease 2026 ")
            .run(&mut ports)
            .expect("a Document");

        assert_eq!(ports.writes, [Write::Document(LEASE)]);
        let edited = ports.document(LEASE);
        assert_eq!(
            (edited.title.as_str(), edited.doc_type, edited.date),
            ("Lease 2026", RECEIPTS, today())
        );
    }

    #[test]
    fn edit_refuses_a_blank_title_and_an_unknown_document() {
        let mut ports = ports();

        ports.refuses(AddError::NoTitle, |ports| edit(LEASE, " ").run(ports));
        ports.refuses(AddError::NotFound, |ports| {
            edit(DocumentId::seed(99), "Gone").run(ports)
        });
    }
}
