//! Personal Ledger TUI entry point. Boots into `Shell`, the status-line/single-view/command-
//! line/keybind-hint-bar chrome ADR-0013
//! (`docs/adr/0013-shell-view-replaces-breadcrumb-app-screen-nav.md`) introduces in place of
//! the breadcrumb `App`/`Screen` stack. That stack is now fully retired: `app`, `screen/` and
//! the `action::Action` message type they alone routed are deleted, so `Shell`/`view/` is the
//! only screen architecture in the crate. The domains whose real CRUD and report logic had
//! only ever lived in `screen/` (Transactions, Budgets, Balance Checks, Reports, CSV import)
//! are wireframe `view/` placeholders until each gets its own redesign pass; recover the old
//! implementation from Git history rather than from a dead module.

mod account;
mod category;
mod db;
mod error;
mod event;
mod format;
mod locale;
mod payee;
mod popup;
mod shell;
mod tag;
mod tui;
mod view;

/// This bin's own Messages, generated at build time from `i18n/<locale>/*.ftl`.
mod msg {
    include!(concat!(env!("OUT_DIR"), "/msg.rs"));
}

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
    let mut config = lib_config::Config::parse(cli.config.path.as_deref())?;
    cli.config.apply_overrides(&mut config);
    let telemetry_level = Some(&config.personal_ledger_config().log());
    let log_file_path = config.personal_ledger_config().log_file_path();
    // Held for the lifetime of `main` -- dropping it stops the background worker that
    // flushes buffered log lines to `log_file_path` (when configured).
    let _log_guard = lib_tracing::init(telemetry_level, log_file_path)?;

    // Resolved once, before the terminal enters raw mode; the Locale never changes at runtime.
    let (requested_locale, locale_source) = config.personal_ledger_config().resolved_locale();
    locale::init(requested_locale, locale_source);

    Shell::with_keybindings(config.keybindings_config().clone())
        .run()
        .await?;

    Ok(())
}
