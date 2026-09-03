//! # Accounts Database Module
//!
//! Data access for the Sync Server's own auth user store (ADR-0010) -- single-account
//! this cycle, holding a username, an Argon2 password hash, and the currently valid
//! refresh-token hash.
//!
//! | Submodule | Purpose |
//! |-----------|---------|
//! | [`model`](model) | Core [`Account`](Account) struct and mock data generation |
//! | [`builder`](builder) | Fluent [`AccountBuilder`](AccountBuilder) for constructing accounts |
//! | [`insert`](insert) | Bootstrap the account into the store |
//! | [`find`](find) | Look up by username, or fetch the single bootstrap account |
//! | [`update`](update) | Rotate/clear the current refresh-token hash |

#![allow(unused)] // For development only

mod builder;
mod find;
mod insert;
mod model;
mod update;

/// Database row model representing one persisted account.
pub use model::Account;

/// Fluent builder for constructing [`Account`] instances.
#[allow(unused)]
pub use builder::AccountBuilder;
