//! # Accounts Database Module
//!
//! Data access for the `accounts` table — the domain Account entity (Cash/Bank/Credit
//! Card/Investment/Loan, FR.10-15, CC-TUI-008). Not the Sync Server's own auth credential;
//! see `CONTEXT.md`'s SyncUser glossary entry for that.
//!
//! | Submodule | Purpose |
//! |-----------|---------|
//! | [`model`](model) | Core [`Accounts`](Accounts) struct and mock data generation |
//! | [`builder`](builder) | Fluent [`AccountsBuilder`](AccountsBuilder) for constructing Accounts |
//! | [`insert`](insert) | Insert a new Account |
//! | [`find`](find) | Look up Accounts by id, list with pagination and filters |
//! | [`update`](update) | Update an Account's details, or its active flag |
//! | [`delete`](delete) | Delete an Account |
//! | [`balance`](balance) | Compute an Account's current Balance (FR.34) |

#![allow(unused)] // For development only

mod balance;
mod builder;
mod delete;
mod find;
mod insert;
mod model;
mod update;

/// Database row model representing a persisted Account.
pub use model::Accounts;

/// Fluent builder for constructing [`Accounts`] instances.
#[allow(unused)]
pub use builder::AccountsBuilder;
