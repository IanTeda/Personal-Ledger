//! Why creating a Ledger wrote nothing, typed so a Client words each case.

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
    /// The port found something already at the path, perhaps made after the explorer checked the
    /// name. It is left as it was.
    #[error("a file is already at the path")]
    Exists,
    /// The port couldn't make, migrate or write the file, as in a folder that can't be written or
    /// a full disk. Whatever was made of it has been removed.
    #[error("the ledger file couldn't be written")]
    Unwritable,
}

/// This crate's fallible results, as every feature crate's.
pub type Result<T, E = Error> = std::result::Result<T, E>;
