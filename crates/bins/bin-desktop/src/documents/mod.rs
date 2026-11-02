//! The Documents module on the Desktop: the page, running on `lib-documents`' Presenters (`view`,
//! `router`), the ports the Presenters read and write through (`ports`, `files`), and the legacy
//! Dialogs (Add, Edit, Import, Edit facts, Accept all and the link picker: `form`, `picker`,
//! `apply`, drawn by `view::dialogs`), which write through the Ledger's forwarding shim until they
//! move onto Presenters (#968). The `gpui`-free model, rules and stub seed live in `lib_documents`
//! and are re-exported here, so every `documents::X` path keeps working.

pub(crate) mod apply;
pub(crate) mod files;
pub(crate) mod form;
pub(crate) mod picker;
pub(crate) mod ports;
pub mod router;
pub(crate) mod types;
pub mod view;

pub use lib_documents::*;
