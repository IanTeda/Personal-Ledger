//! The Documents intents, one Interactor each: it reads the Documents and Document Types from the
//! [`DocumentRepository`], applies the rule this crate holds on a copy, and writes each Document or
//! type the rule added or changed. A refusal writes nothing: the rule runs on the copy, and nothing
//! is written until it lands.
//!
//! Filing writes only Documents, so it is here rather than in a cross-domain module: Accept reads
//! the Transactions through [`DocumentTransactions`], Accept all strong reads the Payees too, and
//! Import reaches the disk through [`DocumentFiles`]. Skip has no Interactor: it is the Client's
//! session state and writes nothing.

mod accept;
mod accept_all_strong;
mod add_document;
mod add_document_type;
mod edit_document;
mod edit_document_type;
mod file_by_hand;
mod import_documents;
mod move_document_type;
mod remove_document_type;
mod set_facts;
mod support;
#[cfg(test)]
mod test_support;
mod toggle_link;
mod undo_filing;

pub use accept::Accept;
pub use accept_all_strong::AcceptAllStrong;
pub use add_document::AddDocument;
pub use add_document_type::AddDocumentType;
pub use edit_document::EditDocument;
pub use edit_document_type::EditDocumentType;
pub use file_by_hand::FileByHand;
pub use import_documents::ImportDocuments;
pub use move_document_type::MoveDocumentType;
pub use remove_document_type::RemoveDocumentType;
pub use set_facts::SetFacts;
pub use toggle_link::ToggleLink;
pub use undo_filing::UndoFiling;
