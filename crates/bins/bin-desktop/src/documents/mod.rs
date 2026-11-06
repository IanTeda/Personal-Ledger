//! The Documents module on the Desktop: the page and its Dialogs, each running on a `lib-documents`
//! Presenter (`view`, `router`), and the file system the Presenters reach through (`files`). The
//! Documents store they read and write is the one the Document Types page shares
//! (`document_types::ports`). The `gpui`-free model, rules and sample seed live in `lib_documents`
//! and are re-exported here, so every `documents::X` path keeps working.

pub(crate) mod files;
pub mod router;
pub(crate) mod types;
pub mod view;

pub use lib_documents::*;
