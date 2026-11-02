//! The Desktop's half of Documents: the label (`model`, `picker`) and form (`form`, `types`) halves
//! that need the Desktop's messages and dialogs. The `gpui`-free model, rules and stub seed live in
//! `lib_documents` and are re-exported here, so every `documents::X` path keeps working.

pub(crate) mod apply;
pub(crate) mod files;
pub(crate) mod form;
pub(crate) mod model;
pub(crate) mod picker;
pub(crate) mod types;

pub use lib_documents::*;
