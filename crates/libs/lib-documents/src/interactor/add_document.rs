//! `+ Add`: a Filed Document from a path the user already knows the meaning of.

use lib_core::DocumentId;

use super::support::{Refusal, write_documents};
use crate::protocols::DocumentRepository;
use crate::{AddError, NewDocument, add_filed};

/// `+ Add`: a Filed Document from a path the user already knows the meaning of. Returns its id.
#[derive(Debug, Clone, PartialEq)]
pub struct AddDocument(pub NewDocument);

impl AddDocument {
    pub fn run(self, repo: &mut dyn DocumentRepository) -> Result<DocumentId, AddError> {
        let outcome = write_documents(repo, |_, documents| add_filed(documents, self.0));
        match &outcome {
            Ok(id) => tracing::info!(document_id = %id, "Added a Document"),
            Err(refusal) => {
                tracing::info!(refusal = refusal.reason(), "Refused to add a Document");
            }
        }
        outcome
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Filing;
    use crate::interactor::test_support::*;

    #[test]
    fn add_writes_a_filed_document() {
        let mut ports = ports();

        let id = AddDocument(new_document("/tmp/will.pdf", " Will "))
            .run(&mut ports)
            .expect("a new path");

        assert_eq!(ports.writes, [Write::Document(id)]);
        let added = ports.document(id);
        assert_eq!(added.title, "Will");
        assert_eq!(added.filing, Filing::Filed);
        assert_eq!(added.bytes, 2_048);
    }

    #[test]
    fn add_refuses_a_blank_title_a_non_document_and_a_held_path() {
        let mut ports = ports();

        ports.refuses(AddError::NoTitle, |ports| {
            AddDocument(new_document("/tmp/will.pdf", "  ")).run(ports)
        });
        ports.refuses(AddError::UnsupportedFile, |ports| {
            AddDocument(new_document("/tmp/notes.txt", "Notes")).run(ports)
        });
        ports.refuses(
            AddError::AlreadyInLibrary("/work/lease.pdf".into()),
            |ports| AddDocument(new_document("/work/lease.pdf", "Lease again")).run(ports),
        );
    }
}
