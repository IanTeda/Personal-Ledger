//! # Transactions Database Module
//!
//! Data access for the `transactions` table — single-entry Transactions against exactly one
//! Account and one Category (FR.16-21, CC-TUI-009).
//!
//! | Submodule | Purpose |
//! |-----------|---------|
//! | [`model`](model) | Core [`Transactions`](Transactions) struct and mock data generation |
//! | [`builder`](builder) | Fluent [`TransactionsBuilder`](TransactionsBuilder) for constructing Transactions |
//! | [`insert`](insert) | Insert a new Transaction |
//! | [`find`](find) | Look up Transactions by id, list with pagination |
//! | [`update`](update) | Update a Transaction's other fields (Reconciled-locked), or its Status/Flagged marker (always allowed) |
//! | [`delete`](delete) | Delete a Transaction |

#![allow(unused)] // For development only

mod builder;
mod delete;
mod find;
mod insert;
mod model;
mod update;

/// Database row model representing a persisted Transaction.
pub use model::Transactions;

/// Fluent builder for constructing [`Transactions`] instances.
#[allow(unused)]
pub use builder::TransactionsBuilder;
