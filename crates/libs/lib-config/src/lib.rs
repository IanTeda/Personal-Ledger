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
//! - [`general`] - `[General]` section: Settings › General's keys (On start, Start view,
//!   Toasts, the Locale, the data dir and the Ledger file), ADR-0057
//! - [`display`] - `[Display]` section: Settings › Display's keys
//! - [`logs`] - `[Logs]` section: the log level and file, read by all three binaries
//! - [`sync_server`] - Sync-Server-only configuration (bind address, database URI), not
//!   read by Clients
//! - [`theme`] - `[theme]`/`[theme.light]`/`[theme.dark]` Colour Role overrides, read by
//!   the TUI/Desktop Clients
//! - [`keys`] - The key catalogue: every key parsed, and each key Settings writes with its
//!   section, environment variable and typed [`ConfigValue`]
//! - [`writer`] - Writes one key back to its section of the user's file, for Settings
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
mod display;
mod error;
mod general;
mod keybindings;
mod keys;
mod ledger;
mod logs;
mod sync_server;
mod theme;
mod writer;

/// Re-export settings [`Error`] type.
pub use error::Error;

/// Re-export [`Result`] type alias used across configuration module.
pub(crate) type Result<T> = std::result::Result<T, Error>;

/// The top-level application configuration type, shared by all three consumers.
pub use ledger::LedgerConfig as Config;

/// The `[General]`, `[Display]` and `[Logs]` sections, one per Settings page (ADR-0057).
pub use display::{
    DATE_STYLE_DEFAULT, DATE_STYLES, DisplayConfig, date_style_from_key, date_style_key,
};
pub use general::{
    DEFAULT_LOCALE, GeneralConfig, LocaleSource, ON_STARTS, on_start_from_key, on_start_key,
};
pub use logs::LogsConfig;

/// The `status_glyphs` and `row_density` values, `lib_core`'s so the Presenters draw with the same
/// type the file is read into.
pub use lib_core::{RowDensity, StatusGlyphs};

/// The key catalogue: each key Settings writes, its section, environment variable and typed value.
pub use keys::{ConfigKey, ConfigValue, Section};

/// Writes Configuration back to the user's `personal-ledger.conf`, for Settings.
pub use writer::{ConfigWriter, Shadow};

/// Sync-Server-only configuration (bind address and database URI).
pub use sync_server::SyncServerConfig;

/// Keyboard shortcut configuration, read by the TUI/Desktop Clients.
pub use keybindings::KeyBindingConfig;

/// Shared `--config`/`-c` CLI argument, flattened into each binary's own `clap::Parser`.
pub use cli::ConfigArgs;

/// `[theme]` Colour Role overrides, read by the TUI/Desktop Clients.
pub use theme::{InvalidThemeEntry, ThemeConfig};
