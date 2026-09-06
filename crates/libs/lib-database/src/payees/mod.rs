//! # Payees Database Module
//!
//! Data access for the `payees` table — the domain Payee entity (FR.16/18/21a-e,
//! CC-TUI-007). A first-class entity as of
//! [ADR-0012](../../../../docs/adr/0012-payee-entity-with-rename-aliases.md), replacing the
//! earlier free-text `Transactions.payee` column; see [`crate::PayeeAliases`] for how a
//! rename is preserved.
//!
//! | Submodule | Purpose |
//! |-----------|---------|
//! | [`model`](model) | Core [`Payees`](Payees) struct and mock data generation |
//! | [`builder`](builder) | Fluent [`PayeesBuilder`](PayeesBuilder) for constructing Payees |
//! | [`insert`](insert) | Insert a new Payee |
//! | [`find`](find) | Look up Payees by id, name, or list with pagination and filters |
//! | [`update`](update) | Rename a Payee (preserving a Payee Alias), or set its active flag |
//! | [`delete`](delete) | Delete a Payee |
//! | [`resolve`](resolve) | Resolve typed Payee text to a Payee, auto-creating if needed |
//! | [`totals`](totals) | Compute the signed Transaction total per Payee (FR.36) |

mod builder;
mod delete;
mod find;
mod insert;
mod model;
mod resolve;
mod totals;
mod update;

/// Database row model representing a persisted Payee.
pub use model::Payees;

/// Fluent builder for constructing [`Payees`] instances.
pub use builder::PayeesBuilder;
