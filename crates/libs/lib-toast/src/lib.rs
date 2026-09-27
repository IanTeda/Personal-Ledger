//! The Toast model shared by the Desktop and TUI Clients.
//!
//! Owns the Toast Kinds and their lifetimes, the visible stack (at most three, eviction and the
//! `+N more` count), `×N` duplicate merging, dismissal, the session Toast history and choosing
//! what the status-line echo shows when Toasts are off. Pure: no I/O, no drawing and no real
//! clock. The bins clock it with [`Toasts::advance`] and pass in the Toasts Preference, so the
//! rules are tested once and the two Clients cannot drift. Design in `docs/toasts-design.md`.

mod history;
mod kind;
mod toasts;

pub use history::{HISTORY_LIMIT, History, HistoryEntry};
pub use kind::ToastKind;
pub use toasts::{Display, MAX_VISIBLE, Toast, Toasts};
