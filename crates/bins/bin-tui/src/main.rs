//! Personal Ledger TUI entry point. Boots into `Shell`, the status-line/single-view/command-
//! line/keybind-hint-bar chrome ADR-0013
//! (`docs/adr/0013-shell-view-replaces-breadcrumb-app-screen-nav.md`) introduces in place of
//! the breadcrumb `App`/`Screen` stack. `app`/`screen` are kept as workspace modules so they
//! still compile — see the ADR — but are no longer referenced here; each of their real
//! screens is migrated into `view/` behind its own redesign pass.

mod action;
mod app;
mod category;
mod db;
mod error;
mod event;
mod popup;
mod screen;
mod shell;
mod tui;
mod view;

use clap::Parser;
use shell::Shell;

pub use error::Error;

/// Crate Result type alias used across the TUI binary.
///
/// Use `TuiResult<T>` for functions that return `T` or a `TuiError`.
pub type Result<T> = std::result::Result<T, crate::Error>;

/// Personal Ledger TUI.
#[derive(Parser)]
struct Cli {
    #[command(flatten)]
    config: lib_config::ConfigArgs,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = lib_config::Config::parse(cli.config.path.as_deref())?;
    let telemetry_level = Some(&config.telemetry_config().level());
    let log_file_path = config.telemetry_config().log_file_path();
    // Held for the lifetime of `main` -- dropping it stops the background worker that
    // flushes buffered log lines to `log_file_path` (when configured).
    let _log_guard = lib_tracing::init(telemetry_level, log_file_path)?;

    Shell::new().run().await?;

    Ok(())
}
