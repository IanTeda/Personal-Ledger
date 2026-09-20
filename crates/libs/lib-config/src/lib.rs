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
//! - [`personal_ledger`] - `[Personal-Ledger]` section: data dir, database file, log
//!   level/file, shared by all three consumers (superseded `[tracing]`/`TracingConfig`)
//! - [`sync_server`] - Sync-Server-only configuration (bind address, database URI), not
//!   read by Clients
//! - [`keybindings`] - Keyboard shortcut configuration, read by the TUI/Desktop Clients
//! - [`cli`] - Shared `--config`/`-c`/`--data`/`-d`/`--file`/`-f`/`--log`/`-l`/`--locale` CLI
//!   arguments, flattened into each binary's own parser
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
mod keybindings;
mod ledger;
mod personal_ledger;
mod sync_server;

/// Re-export settings [`Error`] type.
pub use error::Error;

/// Re-export [`Result`] type alias used across configuration module.
pub(crate) type Result<T> = std::result::Result<T, Error>;

/// The top-level application configuration type, shared by all three consumers.
pub use ledger::LedgerConfig as Config;

/// Personal Ledger configuration.
pub use personal_ledger::{DEFAULT_LOCALE, LocaleSource, PersonalLedgerConfig};

/// Sync-Server-only configuration (bind address and database URI).
pub use sync_server::SyncServerConfig;

/// Keyboard shortcut configuration, read by the TUI/Desktop Clients.
pub use keybindings::KeyBindingConfig;

/// Shared `--config`/`-c` CLI argument, flattened into each binary's own `clap::Parser`.
pub use cli::ConfigArgs;
