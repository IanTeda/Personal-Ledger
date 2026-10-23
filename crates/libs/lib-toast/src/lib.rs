//! What the Shell tells the person, shared by the Desktop and TUI Clients: the Toast model, and
//! the [`Notice`]s a module sends through its Router's [`Notify`] to raise a Toast or set the
//! status-line message (ADR-0048). The Glossary keeps the two concepts apart; they share a crate
//! so every feature crate and the Shell may depend on it.
//!
//! Owns the Toast Kinds and their lifetimes, the visible stack (at most three, eviction and the
//! `+N more` count), `×N` duplicate merging, dismissal, the session Toast history and choosing
//! what the status-line echo shows when Toasts are off. Pure: no I/O, no drawing and no real
//! clock. The bins clock it with [`Toasts::advance`] and pass in the Toasts Preference, so the
//! rules are tested once and the two Clients cannot drift. Design in `docs/toasts-design.md`.

mod history;
mod kind;
mod notice;
mod toasts;

pub use history::{HISTORY_LIMIT, History, HistoryEntry};
pub use kind::ToastKind;
pub use notice::{Notice, Notify};
pub use toasts::{Display, MAX_VISIBLE, Toast, Toasts};
