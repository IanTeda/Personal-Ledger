//! Top-level configuration module.
//!
//! This module organises all configuration-related submodules and re-exports the public
//! domain types used throughout the application: `LedgerConfig`, `SyncServerConfig`, the
//! shared `ConfigArgs` CLI argument group, and the `ConfigError` type used for configuration
//! loading and validation errors.
//!
//! ## Structure
//!
//! - [`error`] - Configuration error types
//! - [`ledger`] - Top-level application configuration, shared by all three consumers
//! - [`database`] - Database connection pool configuration, re-exported by `lib-database`
//! - [`sync_server`] - Sync-Server-only configuration (bind address), not read by Clients
//! - [`tracing`] - Tracing/telemetry configuration
//! - [`cli`] - Shared `--config`/`-c` CLI argument, flattened into each binary's own parser
//!
//! ## Client vs Sync Server
//!
//! Per ADR-0014, `bin-tui`/`bin-desktop` call [`LedgerConfig::parse`], which searches the
//! full defaults/system/user/executable-directory/working-directory/explicit-path/env chain.
//! `bin-sync-server` calls [`LedgerConfig::parse_for_sync_server`] instead, which only
//! applies defaults, an explicit path, and environment variables -- the system/user/
//! executable-directory/working-directory tiers don't correspond to anything meaningful
//! inside a Docker container.

mod cli;
mod database;
mod error;
mod ledger;
mod sync_server;
mod tracing;

/// Re-export settings [`Error`] type.
pub use error::Error;

/// Re-export [`Result`] type alias used across configuration module.
pub type Result<T> = std::result::Result<T, Error>;

/// The top-level application configuration type, shared by all three consumers.
pub use ledger::LedgerConfig;

/// Database connection pool configuration, re-exported by `lib-database` (its own
/// connection pooling is the only consumer of the type, but the type itself lives here
/// alongside `TracingConfig`/`SyncServerConfig` so all layered-config sections stay in one
/// crate).
pub use database::DatabaseConfig;

/// Sync-Server-only configuration (currently just the bind address).
pub use sync_server::SyncServerConfig;

/// Telemetry configuration.
pub use tracing::TracingConfig;

/// Shared `--config`/`-c` CLI argument, flattened into each binary's own `clap::Parser`.
pub use cli::ConfigArgs;
