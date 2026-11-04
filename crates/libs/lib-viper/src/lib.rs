//! The generic types every VIPER module's Presenters and keymap use
//! (`docs/development/viper-architecture.md`): a module's keymap, the field, select and
//! Dialog views its view models are built from, the input and Router a module Dialog's
//! Presenter takes, and the Display Preferences a Presenter draws with.
//!
//! Its own crate rather than part of the first feature crate that needed them (`lib-units`), so
//! the next module imports them instead of copying them, and no feature crate depends on another
//! for them. Not a feature crate: it holds no domain, and like the feature crates it has no UI
//! toolkit and no I/O, so both Clients draw from it.

pub mod dialog;
pub mod display;
pub mod keymap;
pub mod view;
