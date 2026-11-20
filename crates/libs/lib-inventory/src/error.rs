//! The Inventory Refusals (`GLOSSARY.md`): one variant per rule a Location, Area, Item or cover
//! write enforces. A name or cover the Entity refuses keeps its own small error beside its type
//! (`NameError`, `CoverError`), wrapped here. The `#[error]` text is for logs; the person sees
//! the Presenter's `msg::` Message.

use crate::{CoverError, NameError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The Location is no longer on the Ledger.
    #[error("location not found")]
    UnknownLocation,
    /// The Area is no longer on the Ledger, or not one of the Location's.
    #[error("area not found")]
    UnknownArea,
    /// The Item is no longer on the Ledger.
    #[error("item not found")]
    UnknownItem,
    /// A Location or Area name the rules refuse.
    #[error(transparent)]
    Name(#[from] NameError),
    /// A Location's cover the rules refuse.
    #[error(transparent)]
    Cover(#[from] CoverError),
    #[error("an item needs a name")]
    ItemNameEmpty,
    /// A replacement value below zero.
    #[error("an item's replacement value can't be below zero")]
    ReplacementNegative,
    /// The Area holds Items and another Area of the Location must take them.
    #[error("the area holds items and needs a destination")]
    NeedsDestination,
    /// The destination is the Area itself, or not an Area of the same Location.
    #[error("the destination is not another area of the location")]
    BadDestination,
    /// The Area holds Items and is the Location's only Area, so there is nowhere to move them.
    #[error("the location's only area holds items")]
    LastAreaHoldsItems,
}

pub type Result<T> = std::result::Result<T, Error>;
