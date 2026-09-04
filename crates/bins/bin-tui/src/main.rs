//! Personal Ledger TUI entry point. The Dashboard ("Decide TUI screen map and navigation
//! shape") is the navigation hub every entity/report area drills into; see the "TUI App
//! concept" Wayfinder map (issue #61) for the Concept-cycle build-out, and the closed "TUI
//! App feasibility" map (issue #7) for the chart/table widgets it builds on.

mod action;
mod app;
mod db;
mod event;
mod screen;
mod tui;

use app::App;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = lib_config::LedgerConfig::parse(None)?;
    let telemetry_level = Some(&config.telemetry_config().telemetry_level());
    lib_telemetry::init(telemetry_level)?;

    App::new().run().await?;

    Ok(())
}
