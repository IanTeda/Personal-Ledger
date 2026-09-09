//! # Preferences Database Module
//!
//! Data access for the `preferences` table -- Ledger-scoped settings a user edits from
//! inside a running Client (ADR-0014): the default Unit for new Accounts, colour theme,
//! date format, and decimal/thousands separator. Singleton by convention (mirroring
//! `sync_users`), not a database constraint.
//!
//! | Submodule | Purpose |
//! |-----------|---------|
//! | [`model`](model) | Core [`Preferences`](Preferences) struct and mock data generation |
//! | [`builder`](builder) | Fluent [`PreferencesBuilder`](PreferencesBuilder) for constructing Preferences |
//! | [`find`](find) | [`Preferences::find_only`](Preferences::find_only) -- the singleton row, if it exists |
//! | [`update`](update) | [`Preferences::get_or_create_default`](Preferences::get_or_create_default) and [`Preferences::update`](Preferences::update) |
//!
//! No `delete.rs` -- the row is never deleted, only updated.
//!
//! No Change Set emission here: this table is sync-ready in shape (a normal `RowID`, an
//! `updated_on` trigger) but not wired into any sync mechanism yet -- no entity currently
//! emits a Change Set on write, not even `units`/`accounts`.

#![allow(unused)] // For development only

mod builder;
mod find;
mod model;
mod update;

/// Database row model representing the singleton Preferences row.
pub use model::Preferences;

/// Fluent builder for constructing [`Preferences`] instances.
#[allow(unused)]
pub use builder::PreferencesBuilder;
