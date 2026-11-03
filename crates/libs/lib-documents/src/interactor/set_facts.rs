//! The Inbox's Edit: overwrites an Unfiled Document's Extracted Facts.

use lib_core::DocumentId;

use super::support::{logged_document, write_documents};
use crate::protocols::DocumentRepository;
use crate::{ExtractedFacts, FilingError, edit_facts};

/// The Inbox's Edit: overwrites Unfiled Document `id`'s Extracted Facts.
#[derive(Debug, Clone, PartialEq)]
pub struct SetFacts {
    pub id: DocumentId,
    pub facts: ExtractedFacts,
}

impl SetFacts {
    pub fn run(self, repo: &mut dyn DocumentRepository) -> Result<(), FilingError> {
        logged_document(
            "set facts",
            self.id,
            write_documents(repo, |_, documents| {
                edit_facts(documents, self.id, self.facts)
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::interactor::test_support::*;

    #[test]
    fn set_facts_overwrites_an_unfiled_documents_facts() {
        let mut ports = ports();
        let facts = ExtractedFacts {
            merchant: Some("Aldi".into()),
            ..ExtractedFacts::default()
        };

        SetFacts {
            id: UNMATCHED,
            facts: facts.clone(),
        }
        .run(&mut ports)
        .expect("an Unfiled Document");

        assert_eq!(ports.writes, [Write::Document(UNMATCHED)]);
        let intake = ports.document(UNMATCHED).intake.expect("an Intake");
        assert_eq!(intake.facts, facts);
    }

    #[test]
    fn set_facts_refuses_a_filed_and_an_unknown_document() {
        let mut ports = ports();
        let set = |id| SetFacts {
            id,
            facts: ExtractedFacts::default(),
        };

        ports.refuses(FilingError::AlreadyFiled, |ports| set(LEASE).run(ports));
        ports.refuses(FilingError::NotFound, |ports| {
            set(DocumentId::seed(99)).run(ports)
        });
    }
}
