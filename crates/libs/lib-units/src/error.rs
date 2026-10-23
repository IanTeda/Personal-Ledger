//! The Units Refusals (`GLOSSARY.md`): one variant per rule an Interactor enforces, named for what
//! the rule forbids. A variant carries what the Presenter needs for its Message, never English: the
//! `#[error]` text is for logs, and the person sees the Presenter's `msg::` Message.
//!
//! Storage failures aren't here. The ports are infallible (ADR-0046), so a failed write reaches the
//! person as an error Toast through the writer channel, never through an Interactor.

use crate::UnitUse;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The Unit a request addressed is gone, which only happens when it was removed while its
    /// Dialog was open.
    #[error("unit not found")]
    UnitGone,
    /// The Price Source a request or a link names is gone, which only happens when it was deleted
    /// while its Dialog was open.
    #[error("price source not found")]
    PriceSourceGone,
    /// The quote Unit isn't on the Ledger; storage keeps every quote on a Unit.
    #[error("quote unit not found")]
    QuoteGone,
    /// The Ledger always has a base Unit, so its last Unit stays.
    #[error("the only unit can't be deleted")]
    OnlyUnit,
    /// An Account, a Bill Plan or a Budget names the Unit, so it can only be deactivated.
    #[error("a unit in use can only be deactivated")]
    UnitInUse(UnitUse),
    /// The Unit is the base Unit and no new base was given.
    #[error("the base unit needs a new base unit before it can be deleted")]
    BaseUnitNeeded,
    /// The new base Unit is the Unit being deleted, or isn't on the Ledger.
    #[error("new base unit not found")]
    NewBaseGone,
    /// The base Unit is always active, so a deactivated Unit can't become it.
    #[error("a deactivated unit can't be the base unit")]
    InactiveBase,
    /// This many other Units are priced in the Unit, and a Unit's quote must stay on the Ledger.
    /// Re-quoting them would drop their last prices, so the person edits them first.
    #[error("{0} other units are quoted in this unit")]
    QuotedBy(usize),
    /// The base Unit is what new Accounts default to and what the net worth is figured in, so it
    /// stays active until another Unit takes over as base.
    #[error("the base unit can't be deactivated")]
    DeactivateBase,
    /// A built-in Price Source is always listed.
    #[error("a built-in price source can't be deleted")]
    DeleteBuiltIn,
    /// Manual entry is the fallback every Unit can sit on, and has nothing to edit.
    #[error("manual entry can't be edited or deleted")]
    ChangeManualEntry,
    /// A built-in's address, request, price path and auth are fixed in code.
    #[error("a built-in price source's endpoint can't be edited")]
    BuiltInEndpoint,
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
