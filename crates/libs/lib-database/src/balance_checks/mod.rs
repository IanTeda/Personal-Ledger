//! # Balance Checks Database Module
//!
//! Data access for the `balance_checks` table — a point-in-time assertion of what an
//! Account's Balance should be (FR.28-32, CC-TUI-013). Manual entry (this module's `insert`)
//! and CSV import (FR.33, CC-TUI-012, `csv_import`) are the two ways a Balance Check gets
//! created.
//!
//! | Submodule | Purpose |
//! |-----------|---------|
//! | [`model`](model) | Core [`BalanceChecks`](BalanceChecks) struct and mock data generation |
//! | [`builder`](builder) | Fluent [`BalanceChecksBuilder`](BalanceChecksBuilder) |
//! | [`insert`](insert) | Insert a new Balance Check |
//! | [`find`](find) | Look up Balance Checks by id, list with pagination |
//! | [`update`](update) | Update a Balance Check's date or asserted balance |
//! | [`delete`](delete) | Delete a Balance Check |
//! | [`csv_import`](csv_import) | Atomically import many Balance Checks from a CSV file |

mod builder;
mod csv_import;
mod delete;
mod find;
mod insert;
mod model;
mod update;

/// Database row model representing a persisted Balance Check.
pub use model::BalanceChecks;

/// Fluent builder for constructing [`BalanceChecks`] instances.
pub use builder::BalanceChecksBuilder;
