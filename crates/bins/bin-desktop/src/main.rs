//! Personal Ledger desktop entry point: config, tracing, then a call into the lib, where the
//! window and `Shell` setup live so integration tests can share them.

use bin_desktop::Result;
use clap::Parser;

/// Personal Ledger Desktop.
#[derive(Parser)]
struct Cli {
    #[command(flatten)]
    config: lib_config::ConfigArgs,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut config = lib_config::Config::parse(cli.config.path.as_deref())?;
    cli.config.apply_overrides(&mut config);
    let telemetry_level = Some(&config.personal_ledger_config().log());
    let log_file_path = config.personal_ledger_config().log_file_path();
    // Held for the lifetime of `main` -- dropping it stops the background worker that
    // flushes buffered log lines to `log_file_path` (when configured).
    // The Tracing page's live view of this run's events.
    let logs = lib_tracing::LogBuffer::new(lib_tracing::LOG_CAPACITY);
    let _log_guard = lib_tracing::init(telemetry_level, log_file_path, Some(logs.clone()))?;
    bin_desktop::run(&config, logs);
    Ok(())
}
