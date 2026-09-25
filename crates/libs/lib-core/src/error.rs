//! Crate-level error type for `lib-core`.
//!
//! This module wraps all per-type errors from the domain layer, allowing
//! functions to return a single unified `Error` type rather than needing
//! to handle multiple error variants.

use crate::{
    AccountTypeError, BudgetPeriodError, CategoryTypesError, DateStyleError, HexColorError,
    HybridLogicalClockError, MoneyError, RowIDError, TransactionStatusError, UnitKindError,
    UrlSlugError,
};

/// Unified error type for domain operations in `lib-core`.
///
/// This enum wraps all possible errors that can occur in the domain layer,
/// including parsing, validation, and type conversion failures.
#[derive(thiserror::Error, Debug, Clone)]
pub enum Error {
    /// Error from [`RowID`](crate::RowID) operations.
    #[error("RowID error: {0}")]
    RowID(#[from] RowIDError),

    /// Error from [`CategoryTypes`](crate::CategoryTypes) operations.
    #[error("CategoryTypes error: {0}")]
    CategoryTypes(#[from] CategoryTypesError),

    /// Error from [`UnitKind`](crate::UnitKind) operations.
    #[error("UnitKind error: {0}")]
    UnitKind(#[from] UnitKindError),

    /// Error from [`AccountType`](crate::AccountType) operations.
    #[error("AccountType error: {0}")]
    AccountType(#[from] AccountTypeError),

    /// Error from [`Money`](crate::Money) operations.
    #[error("Money error: {0}")]
    Money(#[from] MoneyError),

    /// Error from [`TransactionStatus`](crate::TransactionStatus) operations.
    #[error("TransactionStatus error: {0}")]
    TransactionStatus(#[from] TransactionStatusError),

    /// Error from [`BudgetPeriod`](crate::BudgetPeriod) operations.
    #[error("BudgetPeriod error: {0}")]
    BudgetPeriod(#[from] BudgetPeriodError),

    /// Error from [`UrlSlug`](crate::UrlSlug) operations.
    #[error("UrlSlug error: {0}")]
    UrlSlug(#[from] UrlSlugError),

    /// Error from [`HexColor`](crate::HexColor) operations.
    #[error("HexColor error: {0}")]
    HexColor(#[from] HexColorError),

    /// Error from [`DateStyle`](crate::DateStyle) operations.
    #[error("DateStyle error: {0}")]
    DateStyle(#[from] DateStyleError),

    /// Error from [`HybridLogicalClock`](crate::HybridLogicalClock) operations.
    #[error("HybridLogicalClock error: {0}")]
    HybridLogicalClock(#[from] HybridLogicalClockError),
}

/// Result type alias for domain operations.
///
/// Use `Result<T>` for functions that return `T` or a domain error.
pub type Result<T> = std::result::Result<T, Error>;
