//! Personal Ledger TUI entry point. Boots into `Shell`, the status-line/single-view/command-
//! line/keybind-hint-bar chrome ADR-0013
//! (`docs/adr/0013-shell-view-replaces-breadcrumb-app-screen-nav.md`) introduces in place of
//! the breadcrumb `App`/`Screen` stack. `app`/`screen` are kept as workspace modules so they
//! still compile — see the ADR — but are no longer referenced here; each of their real
//! screens is migrated into `view/` behind its own redesign pass.

mod action;
mod app;
mod db;
mod event;
mod screen;
mod shell;
mod tui;
mod view;

use shell::Shell;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = lib_config::LedgerConfig::parse(None)?;
    let telemetry_level = Some(&config.telemetry_config().telemetry_level());
    lib_telemetry::init(telemetry_level)?;

    Shell::new().run().await?;

    Ok(())
}
