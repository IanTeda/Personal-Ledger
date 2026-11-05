//! Why a Category delete refused, typed so the Dialog words each case.

/// A delete that wrote nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The Category is no longer on the Ledger.
    #[error("category not found")]
    NotFound,
    /// Only leaves are deleted; a parent's children go first.
    #[error("delete or move its children first")]
    HasChildren,
    /// Uncategorised is where a deleted Category's rows go, so it can't go while anything uses it.
    #[error("Uncategorised can't be deleted while anything uses it")]
    UncategorisedInUse,
}

/// This crate's fallible results, as every feature crate's.
pub type Result<T, E = Error> = std::result::Result<T, E>;
