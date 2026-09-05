//! # Payee Aliases Database Module
//!
//! Data access for the `payee_aliases` table — a Payee's former names, preserved
//! automatically by [`crate::Payees::rename`] (see
//! [ADR-0012](../../../../docs/adr/0012-payee-entity-with-rename-aliases.md)). Write-once:
//! there is no `insert`/`update`/`delete` here, since a row is only ever created by a
//! rename and never changed afterward — only [`find`](find) queries are needed.
//!
//! | Submodule | Purpose |
//! |-----------|---------|
//! | [`model`](model) | Core [`PayeeAliases`](PayeeAliases) struct |
//! | [`find`](find) | List every alias, or every alias for one Payee |

mod find;
mod model;

/// Database row model representing a persisted Payee Alias.
pub use model::PayeeAliases;
