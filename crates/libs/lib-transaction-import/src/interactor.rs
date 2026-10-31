//! The Import's two Interactors: [`MatchStatement`] reads the statement and matches each line,
//! and [`CommitImport`] checks the rows and the Account, resolves the plan and sends it through
//! the port. Moving through the rows, opening a select and choosing are the Presenter's state
//! edits, not Interactors.
//!
//! A refusal is a variant of the crate's one [`Error`](crate::Error), which the Presenter words.

mod commit_import;
mod match_statement;

pub use commit_import::CommitImport;
pub use match_statement::{MatchStatement, Matched};
