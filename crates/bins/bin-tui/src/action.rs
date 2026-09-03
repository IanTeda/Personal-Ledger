//! Actions the application can perform.
//!
//! An [`Action`] is the single message type routed through [`App::update`](crate::app::App::update)
//! — the hybrid Elm/Component architecture locked in by ADR-0003
//! (`docs/adr/0003-hybrid-tea-component-tui-architecture.md`). Input events and background
//! tasks both produce `Action`s onto the same channel rather than mutating state directly.

/// A message the application reacts to, however it originated (keyboard input, a periodic
/// tick, or a background task reporting a result).
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// A periodic tick, driving redraws even without input (e.g. for future animated charts).
    Tick,
    /// The user asked to quit (`q` or `Esc`).
    Quit,
    /// The user asked to switch to the next screen (`Tab`).
    NextScreen,
    /// The user asked to switch to the previous screen (`Shift`+`Tab`).
    PrevScreen,
    /// The embedded-SQLite feasibility demo (FC-TUI-005) finished loading real category data.
    CategoriesLoaded(Vec<lib_database::Categories>),
    /// The embedded-SQLite feasibility demo (FC-TUI-005) failed to load real category data.
    CategoriesLoadFailed(String),
}
