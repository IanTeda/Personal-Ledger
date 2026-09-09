//! # Shared CLI Arguments
//!
//! A reusable `clap::Args` group for locating an explicit configuration file, meant to be
//! `#[command(flatten)]`-ed into each binary's own `clap::Parser` struct (rather than each of
//! `bin-tui`/`bin-desktop`/`bin-sync-server` redefining the same `--config`/`-c` flag). Kept
//! as `Args`, not `Parser`, so each binary's own top-level struct still picks up its own
//! name/version from its own crate at compile time.

/// Shared "where's the config file" argument.
#[derive(Debug, Clone, Default, clap::Args)]
pub struct ConfigArgs {
    /// Path to an explicit configuration file. Takes precedence over the file-location
    /// search, but is still overridden by environment variables -- see
    /// LedgerConfig::parse's documented precedence order.
    #[arg(short = 'c', long = "config", value_name = "PATH")]
    pub path: Option<std::path::PathBuf>,
}
