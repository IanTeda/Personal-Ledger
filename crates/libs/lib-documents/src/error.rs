//! The Documents Refusals (`GLOSSARY.md`): one variant per rule a Document, filing or Document
//! Type write enforces. A Document Type name the rules refuse keeps its own small error beside its
//! check (`NameError`), wrapped here; a typed path's `PathRefusal` is returned directly, never
//! through `Result<T>`, so it isn't. The `#[error]` text is for logs, and carries no Title or
//! name (ADR-0047); the person sees the Presenter's `msg::` Message.

use crate::types::NameError;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// No Document has that id: it has gone since the Dialog opened.
    #[error("document not found")]
    NotFound,
    /// The Document is already Filed.
    #[error("the document is already filed")]
    AlreadyFiled,
    /// The Transaction to link does not exist.
    #[error("transaction not found")]
    NoSuchTransaction,
    /// Only PDF, JPG and PNG files are Documents.
    #[error("only PDF, JPG and PNG files are documents")]
    UnsupportedFile,
    /// One Document per path: the Library already holds this one, under this Title.
    #[error("the library already holds this file")]
    AlreadyInLibrary(String),
    /// A blank Title.
    #[error("a document needs a title")]
    NoTitle,
    /// The Document Type no longer exists.
    #[error("document type not found")]
    UnknownDocumentType,
    /// Other, the Default Document Type, can't be removed.
    #[error("the default document type can't be removed")]
    DefaultDocumentType,
    /// The Document Type has Filed files and no destination type was given.
    #[error("the document type has filed documents and needs a destination")]
    NeedsDestination,
    /// A Document Type name the rules refuse.
    #[error(transparent)]
    Name(#[from] NameError),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
