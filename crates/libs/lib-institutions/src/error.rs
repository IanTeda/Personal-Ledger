//! The Institutions Refusals (`GLOSSARY.md`): one variant per rule an Institution write enforces.
//! The `#[error]` text is for logs; the person sees the Presenter's `msg::` Message.

use crate::InstitutionUse;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("an institution needs a name")]
    NameRequired,
    /// Carries the name as the Institution holding it spells it.
    #[error("another institution has this name")]
    DuplicateName(String),
    /// The Institution is no longer on the Ledger.
    #[error("the institution is not on the Ledger")]
    NotFound,
    /// Accounts name it; carries them.
    #[error("the institution is used by {} accounts", .0.accounts.len())]
    InUse(InstitutionUse),
}

pub type Result<T> = std::result::Result<T, Error>;
