//! The crate's error type. No function here is fallible yet (Toasts, their history and the status-line echo are in-memory rules over values the caller passes in), so there is nothing to refuse
//! and `Error` has no variants.

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {}

pub type Result<T, E = Error> = std::result::Result<T, E>;
