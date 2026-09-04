//! # Units Database Module
//!
//! Data access for the `units` table — the currencies, cryptocurrencies, stocks, and other
//! tradeable instruments Accounts and Transactions are denominated in (FR.1-3, scoped down
//! to plain CRUD; the exchange-rate/pricing parts of FR.2-3 are superseded by the "no
//! cross-Unit conversion in V1" decision — see `docs/product-requirements.md`).
//!
//! | Submodule | Purpose |
//! |-----------|---------|
//! | [`model`](model) | Core [`Units`](Units) struct and mock data generation |
//! | [`builder`](builder) | Fluent [`UnitsBuilder`](UnitsBuilder) for constructing Units |
//! | [`insert`](insert) | Insert a new Unit |
//! | [`find`](find) | Look up Units by id/code, list with pagination and filters |
//! | [`update`](update) | Update a Unit's details, or its active flag |
//! | [`delete`](delete) | Delete a Unit individually or in batch |

#![allow(unused)] // For development only

mod builder;
mod delete;
mod find;
mod insert;
mod model;
mod update;

/// Database row model representing a persisted Unit.
pub use model::Units;

/// Fluent builder for constructing [`Units`] instances.
#[allow(unused)]
pub use builder::UnitsBuilder;
