//! The Documents Refusals (`GLOSSARY.md`): one variant per rule a Document, filing or Document
//! Type write enforces. A typed path or a Document Type name the rules refuse keeps its own small
//! error beside its check (`PathRefusal`, `NameError`), wrapped here. The `#[error]` text is for
//! logs; the person sees the Presenter's `msg::` Message.

use crate::PathRefusal;
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
    /// A typed path the rules refuse.
    #[error(transparent)]
    Path(#[from] PathRefusal),
    /// A Document Type name the rules refuse.
    #[error(transparent)]
    Name(#[from] NameError),
}

pub type Result<T> = std::result::Result<T, Error>;
