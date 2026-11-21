//! The Payees Refusals (`GLOSSARY.md`): one variant per rule a Payee write enforces. A variant
//! carries the names the Presenter's `msg::` Message needs, but the `#[error]` text never repeats
//! them: it is for logs, which never hold a Payee's name.

use crate::PayeeUse;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("a payee needs a name")]
    NameRequired,
    /// Another Payee already has this name (case-insensitively); carries that Payee's name.
    #[error("another payee has this name")]
    DuplicateName(String),
    /// Another Payee already owns this alias.
    #[error("another payee has this match rule")]
    AliasTaken { alias: String, owner: String },
    #[error("payee not found")]
    NotFound,
    /// A Split or a Bill Plan names the Payee, so it can only be deactivated.
    #[error("a payee in use can only be deactivated")]
    InUse(PayeeUse),
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
