//! The Shell: the frame every View and module is hosted in, as a toolkit-free VIPER module
//! (`docs/development/viper-architecture.md`, ADR-0048). Its Presenter decides what the frame
//! does and asks the Client's root Router to act; the Client keeps every timer and delivers each
//! tick as an input, so the Presenter never reads time itself and its tests stay deterministic.
//!
//! It grows ticket by ticket (#820). Today it holds the [`Clock`](lib_core::Clock) and the month
//! turn: on a [`ShellInput::DayTick`] that reads a new calendar month, it calls
//! [`ShellRouter::month_turned`]. It knows whether a Dialog is open, and holds the Dialog keymap
//! every Dialog's keys resolve through ([`dialog`]). It holds the global keymap and resolves each
//! key the Client offers it ([`keymap`], [`ShellPresenter::key`]), with the `g` prefix the Client's
//! timer ends ([`ShellInput::PrefixExpired`]), and owns the [`Noun`] and [`ActiveView`] the Client
//! navigates by ([`navigation`]). The Client's other inputs, a Notice from a View not yet a module,
//! a legacy write and a failed write, pass through it to the Router. No module depends on it; only
//! the bins do.

pub mod dialog;
pub mod fake;
pub mod keymap;
pub mod navigation;
pub mod presenter;
pub mod protocols;

pub use dialog::{DialogCommand, dialog_keymap};
pub use keymap::{ShellCommand, global_keymap};
pub use navigation::{ActiveView, Noun};
pub use presenter::{DialogInput, ShellInput, ShellKey, ShellPresenter};
pub use protocols::ShellRouter;
