//! The Accounts Refusals (`GLOSSARY.md`): one variant per rule an Account write enforces, named for
//! what the rule forbids. The `#[error]` text is for logs; the person sees the Presenter's `msg::`
//! Message.
//!
//! Storage failures aren't here. The ports are infallible (ADR-0046), so a failed write reaches the
//! person as an error Toast through the writer channel, never through an Interactor.

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The Account a request addressed is gone, which only happens when it was deleted while its
    /// Dialog was open.
    #[error("account not found")]
    NotFound,
    /// A form was confirmed without everything an Account needs.
    #[error("the account details are incomplete")]
    Incomplete,
}

pub type Result<T> = std::result::Result<T, Error>;
