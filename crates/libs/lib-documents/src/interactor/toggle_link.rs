//! Adds a Link to a Document, or removes it when it has it.

use lib_core::DocumentId;

use super::support::{logged_document, write_documents};
use crate::protocols::DocumentRepository;
use crate::{DocumentLink, FilingError, toggle_link};

/// Adds `link` to Document `id`, or removes it when it has it. Returns whether it was added.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToggleLink {
    pub id: DocumentId,
    pub link: DocumentLink,
}

impl ToggleLink {
    pub fn run(self, repo: &mut dyn DocumentRepository) -> Result<bool, FilingError> {
        logged_document(
            "toggle link",
            self.id,
            write_documents(repo, |_, documents| {
                toggle_link(documents, self.id, self.link).ok_or(FilingError::NotFound)
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interactor::test_support::*;
    use lib_core::PayeeId;

    #[test]
    fn toggle_link_adds_a_link_then_removes_it() {
        let mut ports = ports();
        let toggle = ToggleLink {
            id: LEASE,
            link: DocumentLink::Payee(PayeeId::seed(1)),
        };

        assert_eq!(toggle.run(&mut ports), Ok(true));
        assert_eq!(ports.document(LEASE).links, [toggle.link]);
        assert_eq!(toggle.run(&mut ports), Ok(false));
        assert!(ports.document(LEASE).links.is_empty());
        assert_eq!(
            ports.writes,
            [Write::Document(LEASE), Write::Document(LEASE)]
        );
    }

    #[test]
    fn toggle_link_refuses_an_unknown_document() {
        let mut ports = ports();

        ports.refuses(FilingError::NotFound, |ports| {
            ToggleLink {
                id: DocumentId::seed(99),
                link: DocumentLink::Payee(PayeeId::seed(1)),
            }
            .run(ports)
        });
    }
}
