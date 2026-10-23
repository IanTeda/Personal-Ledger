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
/// resolved. Takes only what the shell reads so tests can build one without parsing the
/// user's config, and never touches the locale or the terminal. `stores` is where the Units are
/// kept, the Ledger file's adapter or a test's in-memory ports, with the commit that ends each
/// Interactor call's writes, every adapter's at once, and the writer's failure channel, on which
/// a failed write rolls back. The TUI has no Transactions or Bills View on storage yet (#774), so
/// it leaves those stores unread.
pub fn build_shell(
    keybindings: KeyBindingConfig,
    colours: colours::Colours,
    stores: lib_database::Stores,
) -> Shell {
    Shell::with_keybindings(keybindings)
        .with_units(stores.units, stores.commit, stores.failures)
        .with_colours(colours)
}
