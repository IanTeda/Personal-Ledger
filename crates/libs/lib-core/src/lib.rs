//! # Domain Types Module
//!
//! This module contains the core domain types and business logic abstractions
//! for the Personal Ledger backend. Domain types represent the fundamental
//! business concepts and enforce invariants at the type level.
//!
//! The domain layer is independent of external frameworks and focuses on
//! expressing business rules clearly and safely using Rust's type system.
//!
//! ## Available Types
//!
//! - [`CategoryTypes`] - Classification types for financial transactions
//! - [`RowID`] - Time-ordered UUID v7 identifiers for database rows
//! - [`UrlSlug`] - URL-safe identifiers for web-friendly resource names
//! - [`HexColor`] - Validated hexadecimal RGB colour representation
//!
//! ## Design Principles
//!
//! - **Type Safety**: Use newtypes to prevent mixing of different ID types
//! - **Validation**: Enforce business rules at construction time
//! - **Immutability**: Domain objects are immutable after creation
//! - **Serialisation**: All types support JSON serialisation for API responses
//!
//! ## Note
//!
//! This domain layer is designed for SQLite-backed persistence and does not reference Postgres or other database engines. All type mappings and invariants are implemented for SQLite compatibility.

// TODO: Add domain error tyep

/// Database row identifier type using time-ordered UUID v7.
///
/// [`RowID`] provides unique, sortable identifiers that maintain chronological
/// ordering while ensuring global uniqueness across distributed systems.
/// RowIDs are naturally ordered by creation time and prevent accidental
/// mixing with other identifier types.
mod row_id;
pub use row_id::{RowID, RowIDError};

/// Category types used for classifying financial transactions.
///
/// [`CategoryTypes`] represents the fundamental accounting categories
/// (assets, liabilities, income, expenses, equity) used to classify
/// and organise financial transactions in the personal ledger.
/// These categories follow the standard accounting equation and provide
/// type-safe classification for all financial operations.
mod category_types;
pub use category_types::{CategoryTypes, CategoryTypesError};

mod url_slug;
/// URL-safe slug type for human-readable, SEO-friendly identifiers.
///
/// [`UrlSlug`] creates web-friendly identifiers from titles and names by
/// converting them to lowercase, alphanumeric strings with hyphens. Used
/// throughout the application for creating readable URLs and resource names
/// that are both user-friendly and search engine optimized.
pub use url_slug::{UrlSlug, UrlSlugError};

mod hex_color;
/// Hexadecimal RGB colour type for validated colour values.
///
/// [`HexColor`] guarantees colours remain in canonical `#RRGGBB` format while
/// providing convenient access to individual RGB channels. Useful for
/// theming, categorisation, and any feature that requires precise colour
/// handling across the application.
pub use hex_color::{HexColor, HexColorError};

mod hybrid_logical_clock;
/// Hybrid Logical Clock timestamp type used for Change Set conflict resolution.
///
/// [`HybridLogicalClock`] pairs physical time with a monotonic logical counter, and
/// [`HlcClock`] mints strictly increasing values of it for one Client or the Sync
/// Server -- see [ADR-0009](https://github.com/IanTeda/Personal-Ledger/blob/feasibility/docs/adr/0009-lww-sqlite-change-set-log.md).
pub use hybrid_logical_clock::{HlcClock, HybridLogicalClock, HybridLogicalClockError};
