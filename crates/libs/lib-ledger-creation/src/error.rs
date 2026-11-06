//! Why creating a Ledger wrote nothing, typed so a Client words each case.

use crate::CreationFailure;

/// A create that wrote no Ledger file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The base Unit has no code, which Add unit requires too.
    #[error("the base unit needs a code")]
    CodeNeeded,
    /// The base Unit has no name, which Add unit requires too.
    #[error("the base unit needs a name")]
    NameNeeded,
    /// One of `lib-units`' Add unit rules: the Price Source is gone, or the Unit is quoted in
    /// another, which a new Ledger, holding no other Unit, can't have.
    #[error(transparent)]
    Unit(#[from] lib_units::Error),
    /// The port wrote no file.
    #[error(transparent)]
    Failed(#[from] CreationFailure),
}

/// This crate's fallible results, as every feature crate's.
pub type Result<T, E = Error> = std::result::Result<T, E>;
