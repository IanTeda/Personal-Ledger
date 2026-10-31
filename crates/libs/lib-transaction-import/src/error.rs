//! Why an Import's commit refused, typed so the Presenter words each case. Every refusal writes
//! nothing.

/// A commit that wrote nothing. A variant carries the name or alias the Presenter's Message needs,
/// but the `#[error]` text never repeats it: it is for logs, which never hold a Payee's name.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// Rows still missing a Payee or a Category, however many.
    #[error("{0} rows still need review")]
    NeedsReview(usize),
    #[error("the statement's account is gone")]
    AccountGone,
    #[error("the statement's account is inactive")]
    AccountInactive,
    /// A new Payee's name is another Payee's name or alias, case-insensitively.
    #[error("a new payee's name is taken")]
    PayeeNameTaken(String),
    /// A new Payee's alias is another Payee's, or isn't stored-normalised.
    #[error("a new payee's alias is taken or not normalised")]
    AliasTaken(String),
}

/// This crate's fallible results, as every feature crate's.
pub type Result<T, E = Error> = std::result::Result<T, E>;
