//! Tags: free-form labels a Split carries (`GLOSSARY.md`'s Tag), each with an optional colour. A
//! VIPER module (`docs/development/viper-architecture.md`, #847, #863): the Entity with its rules
//! and sample seed (`entity`), one Interactor per intent (`interactor`), one Presenter per View and
//! Dialog (`presenter`), the ports and the `TagsRouter` (`protocols`), the page's default keys
//! (`keymap`) and its Messages (`msg`). `fake` holds the in-memory ports, the sample seed and a
//! recording Router; `contract` the checks both they and a stored adapter keep (#861). Remove and
//! merge rewrite other domains' Splits, so they are `lib-tag-removal`'s (#1002), whose Dialogs
//! close with this crate's [`TagsOutcome`]. No UI toolkit and no I/O; the Desktop draws the view
//! models (`bin-desktop/src/tags/`).

#[cfg(feature = "contract-tests")]
#[expect(
    clippy::expect_used,
    reason = "a contract check is test code a consumer runs, and fails by panicking as a test does"
)]
pub mod contract;
mod entity;
pub mod fake;
pub mod interactor;
pub mod keymap;
pub mod msg;
pub mod presenter;
pub mod protocols;

pub use entity::*;
pub use protocols::{
    SharedPorts, TagChange, TagRepository, TagUsage, TagUse, TagsDialog, TagsOutcome, TagsPorts,
    TagsRouter,
};
