//! The Shell: the frame every View and module is hosted in, as a toolkit-free VIPER module
//! (`docs/development/viper-architecture.md`, ADR-0048). Its Presenter decides what the frame
//! does and asks the Client's root Router to act; the Client keeps every timer and delivers each
//! tick as an input, so the Presenter never reads time itself and its tests stay deterministic.
//!
//! It grows ticket by ticket (#820). Today it holds only the [`Clock`](lib_core::Clock) and the
//! month turn: on a [`ShellInput::DayTick`] that reads a new calendar month, it calls
//! [`ShellRouter::month_turned`]. No module depends on it; only the bins do.

pub mod fake;
pub mod presenter;
pub mod protocols;

pub use presenter::{ShellInput, ShellPresenter};
pub use protocols::ShellRouter;
