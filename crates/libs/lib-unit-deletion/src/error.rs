//! Why Unit delete refused, typed so the Dialog words each case.

/// A delete that wrote nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// One of `lib-units`' own delete rules, which hold here unchanged but for the Institutions
    /// naming the Unit: this delete reassigns them rather than refusing.
    #[error(transparent)]
    Unit(#[from] lib_units::Error),
    /// A Usual unit chosen for an Institution is the Unit being deleted, is no longer on the
    /// Ledger, or is deactivated, which an Institution's Usual unit select never offers.
    #[error("replacement usual unit not found or deactivated")]
    ReplacementGone,
}

/// This crate's fallible results, as every feature crate's.
pub type Result<T, E = Error> = std::result::Result<T, E>;
