//! Why Tag remove or merge refused, typed so the Dialogs word each case.

/// A remove or merge that wrote nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The Tag, or either Tag of a merge, is no longer on the Ledger.
    #[error("tag not found")]
    NotFound,
    #[error("a tag can't be merged into itself")]
    MergeIntoItself,
}

/// This crate's fallible results, as every feature crate's.
pub type Result<T, E = Error> = std::result::Result<T, E>;
