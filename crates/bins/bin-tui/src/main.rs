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

use bin_tui::{Result, build_shell, colours, locale};
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
    let _log_guard = lib_tracing::init(telemetry_level, log_file_path, None)?;
    config.theme_config().warn_invalid();

    // The Ledger opens before the terminal does: the TUI never runs on a file it can't save to.
    let path = config.personal_ledger_config().file();
    let storage = match lib_database::open_storage(path, chrono::Local::now().date_naive()) {
        Ok(storage) => storage,
        Err(error) => {
            tracing::error!(%error, path = %path.display(), "Failed to open the Ledger");
            return Err(error.into());
        }
    };

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
    // The TUI's Transactions View still draws its fixture and it has no Bills View (#774), so it
    // leaves those adapters unread.
    let lib_database::Storage {
        units,
        transactions: _,
        bills: _,
        pending,
        writer,
        failures,
    } = storage;
    let ran = build_shell(
        config.keybindings_config().clone(),
        colours,
        Box::new(units),
        std::rc::Rc::new(pending),
        failures,
    )
    .run()
    .await;
    // The writer's thread dies with the process, so the last writes land before it does.
    if let Err(error) = writer.flush() {
        tracing::error!(%error, "Failed to flush the Ledger's last writes");
    }
    ran
}
