//! Personal Ledger TUI entry point. Boots into `Shell`, the status-line/single-view/command-
//! line/keybind-hint-bar chrome ADR-0013
//! (`docs/adr/0013-shell-view-replaces-breadcrumb-app-screen-nav.md`) introduces in place of
//! the breadcrumb `App`/`Screen` stack. That stack is now fully retired: `app`, `screen/` and
//! the `action::Action` message type they alone routed are deleted, so `Shell`/`view/` is the
//! only screen architecture in the crate. The domains whose real CRUD and report logic had
//! only ever lived in `screen/` (Transactions, Budgets, Balance Checks, Reports, CSV import)
//! are wireframe `view/` placeholders until each gets its own redesign pass; recover the old
//! implementation from Git history rather than from a dead module.

mod detect;

use bin_tui::{Result, build_shell, colours, locale, state};
use clap::Parser;

/// Personal Ledger TUI.
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
    // Settings › Logs reads this; the capture only fills the buffer, never the terminal the TUI
    // owns in raw mode.
    let logs = lib_tracing::LogBuffer::new(lib_tracing::LOG_CAPACITY);
    let _log_guard = lib_tracing::init(telemetry_level, log_file_path, Some(logs.clone()))?;
    config.theme_config().warn_invalid();

    // Resolved once, before the terminal enters raw mode; the Locale never changes at runtime.
    let (requested_locale, locale_source) = config.personal_ledger_config().resolved_locale();
    locale::init(requested_locale, locale_source);

    // Detected once, before the terminal enters raw mode and crossterm starts reading input,
    // which would otherwise read the probe's reply as key presses.
    let terminal_colours = config.personal_ledger_config().terminal_colours();
    let depth = detect::colour_depth_from_env();
    if depth == colours::ColourDepth::Ansi16 && !terminal_colours {
        tracing::warn!("16-colour terminal: drawing terminal colours instead of the Colour Theme");
    }
    // The fallback table ignores the Colour Variant, so don't make the terminal wait.
    let system = if terminal_colours || depth == colours::ColourDepth::Ansi16 {
        None
    } else {
        detect::system_variant_from_terminal()
    };
    let colours = colours::Colours::new(
        config.theme_config().overrides.clone(),
        terminal_colours,
        depth,
    )
    .with_system(system);
    // No Ledger opens until `:open` or `:new`, whatever On start names: opening it at start is
    // #910's. The Shell flushes the Open Ledger's writes as it quits, and the file it had open is
    // remembered for On start.
    let kept = state::load();
    let file = config.personal_ledger_config().file();
    if let Some(path) =
        lib_core::ledger_to_open(kept.preferences.on_start, file, kept.last_ledger.as_deref())
    {
        tracing::debug!(path = %path.display(), "On start names a Ledger; starting with none open");
    }
    let mut shell = build_shell(config.keybindings_config().clone(), colours, file)
        .with_configuration(config.personal_ledger_config(), config.writer().clone())
        .with_client_preferences(&kept.preferences)
        .with_log_buffer(logs);
    let ran = shell.run().await;
    let quit = state::TuiState {
        preferences: shell.client_preferences(kept.preferences.on_start),
        last_ledger: shell.quit_ledger().map(std::path::Path::to_path_buf),
    };
    if let Err(error) = state::save(&quit) {
        tracing::warn!(%error, "Failed to save the TUI's state");
    }
    ran
}
