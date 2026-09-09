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
//! - [`sync_server`] - Sync-Server-only configuration (bind address), not read by Clients
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
mod error;
mod ledger;
mod sync_server;

/// Configuration loading and validation errors.
pub use error::{ConfigError, ConfigResult};

/// The top-level application configuration type, shared by all three consumers.
pub use ledger::LedgerConfig;

/// Sync-Server-only configuration (currently just the bind address).
pub use sync_server::SyncServerConfig;

/// Shared `--config`/`-c` CLI argument, flattened into each binary's own `clap::Parser`.
pub use cli::ConfigArgs;
