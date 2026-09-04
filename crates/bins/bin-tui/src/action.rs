//! Actions the application can perform.
//!
//! An [`Action`] is the single message type routed through [`App::update`](crate::app::App::update)
//! — the hybrid Elm/Component architecture locked in by ADR-0003
//! (`docs/adr/0003-hybrid-tea-component-tui-architecture.md`). Input events and background
//! tasks both produce `Action`s onto the same channel rather than mutating state directly.

/// Whether the active screen currently wants raw keystrokes treated as global single-letter
/// shortcuts (`Navigation`) or as literal text entry (`Editing`) — the mode split "Decide
/// keybinding and navigation/workflow scheme" locked in, so a free-text field (Payee,
/// description, ...) never fires a shortcut just because it shares a letter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputMode {
    /// List navigation and single-letter shortcuts (`n`, `d`, `/`, `s`, ...) are live.
    #[default]
    Navigation,
    /// A text field has focus; every keystroke is literal input except `Esc`/`Enter`/`Tab`.
    #[allow(dead_code)] // No screen has a text field yet; the first one (issue #67+) will.
    Editing,
}

/// A message the application reacts to, however it originated (keyboard input, a periodic
/// tick, or a background task reporting a result).
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// A periodic tick, driving redraws even without input (e.g. for future animated charts).
    Tick,
    /// `Ctrl+C` from anywhere — the hard-quit safety net, always available regardless of
    /// which screen is active or what `InputMode` it's in.
    Quit,
    /// `Esc` — pops one level of the navigation stack; only becomes [`Action::Quit`]-like
    /// (see `App::update`) when there's nothing left to pop (the dashboard is the base).
    Back,
    /// Push the Settings drill-in screen.
    OpenSettings,
    /// Push the Help screen, listing global keys plus the active screen's own.
    OpenHelp,
    /// The embedded-SQLite feasibility demo (FC-TUI-005) finished loading real category data.
    CategoriesLoaded(Vec<lib_database::Categories>),
    /// The embedded-SQLite feasibility demo (FC-TUI-005) failed to load real category data.
    CategoriesLoadFailed(String),
}
