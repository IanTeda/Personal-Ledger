//! # Sync Users Database Module
//!
//! Data access for the Sync Server's own auth user store (ADR-0010) -- single-account
//! this cycle, holding a username, an Argon2 password hash, and the currently valid
//! refresh-token hash. See `CONTEXT.md`'s SyncUser glossary entry: this is not Ledger
//! data and never syncs via Change Sets.
//!
//! | Submodule | Purpose |
//! |-----------|---------|
//! | [`model`](model) | Core [`SyncUser`](SyncUser) struct and mock data generation |
//! | [`builder`](builder) | Fluent [`SyncUserBuilder`](SyncUserBuilder) for constructing sync users |
//! | [`insert`](insert) | Bootstrap the sync user into the store |
//! | [`find`](find) | Look up by username, or fetch the single bootstrap sync user |
//! | [`update`](update) | Rotate/clear the current refresh-token hash |

#![allow(unused)] // For development only

mod builder;
mod find;
mod insert;
mod model;
mod update;

/// Database row model representing one persisted sync user.
pub use model::SyncUser;

/// Fluent builder for constructing [`SyncUser`] instances.
#[allow(unused)]
pub use builder::SyncUserBuilder;
