//! Personal Ledger TUI, as a library so `tests/` can drive `Shell` the way `main` does
//! (ADR-0035). `main.rs` keeps only the CLI, config, tracing, locale and terminal-colour
//! detection, and hands the rest to this crate.

mod account;
mod category;
pub mod colours;
pub mod error;
pub mod event;
mod fixture;
mod format;
pub mod link;
pub mod locale;
mod payee;
mod popup;
pub mod shell;
mod tag;
mod toast;
mod tui;
pub mod units;
mod view;

/// This bin's own Messages, generated at build time from `i18n/<locale>/*.ftl`.
mod msg {
    include!(concat!(env!("OUT_DIR"), "/msg.rs"));
}

pub use error::Error;

pub use shell::Shell;

use lib_config::KeyBindingConfig;

/// Crate Result type alias used across the TUI.
///
/// Use `TuiResult<T>` for functions that return `T` or a `TuiError`.
pub type Result<T> = std::result::Result<T, Error>;

/// Builds the `Shell` the way `main` does, from the key bindings and the Colours already
/// resolved, with no Ledger open: `:open` and `:new` open a Ledger file, and a bare `:open` opens
/// `ledger_file`, the configured `file`. Takes only what the shell reads so tests can build one
/// without parsing the user's config, and never touches the locale or the terminal.
pub fn build_shell(
    keybindings: KeyBindingConfig,
    colours: colours::Colours,
    ledger_file: Option<&std::path::Path>,
) -> Shell {
    let shell = Shell::with_keybindings(keybindings).with_colours(colours);
    match ledger_file {
        Some(path) => shell.with_ledger_file(path),
        None => shell,
    }
}
