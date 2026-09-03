//! # Change Sets Database Module
//!
//! Provides data access helpers, a builder, and the model for the Sync Server's own
//! durable Change Set log -- the store [ADR-0009](https://github.com/IanTeda/Personal-Ledger/blob/feasibility/docs/adr/0009-lww-sqlite-change-set-log.md)
//! fixed for propagating Client edits at field granularity.
//!
//! ## Overview
//!
//! | Submodule | Purpose |
//! |-----------|---------|
//! | [`model`](model) | Core [`ChangeSet`](ChangeSet) struct and mock data generation |
//! | [`builder`](builder) | Fluent [`ChangeSetBuilder`](ChangeSetBuilder) for constructing Change Sets |
//! | [`insert`](insert) | Insert a Change Set into the log |
//! | [`find`](find) | Query Change Sets since a given cursor (the Sync Server's pull) |

#![allow(unused)] // For development only

mod builder;
mod find;
mod insert;
mod model;

/// Database row model representing one persisted Change Set.
///
/// See the model module for implementation details.
pub use model::ChangeSet;

/// Fluent builder for constructing [`ChangeSet`] instances in tests and Client-side code.
///
/// See the builder module for implementation details.
#[allow(unused)]
pub use builder::ChangeSetBuilder;
