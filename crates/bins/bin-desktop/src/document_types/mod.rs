//! The Document Types module's Desktop half (`docs/development/viper-architecture.md`, #966): the
//! Views for the Settings › Documents page and its Dialogs, and `impl DocumentTypesRouter`. Every
//! rule, every piece of text and the keymap are `lib-documents`'; this folder turns gpui keys and
//! clicks into Presenter events and draws the view models.
//!
//! - `view/page.rs` wraps the page Presenter in an Entity, which the Settings View hosts.
//! - `view/dialog.rs` holds each Dialog's Presenter in an Entity in the Dialog host, one View per
//!   Dialog in `view/dialog/`.
//! - `router.rs` is the Router and the module's entry point ([`document_types_page`]).
//! - `ports.rs` holds the Documents store as the Presenters share it.
//!
//! The Documents destination (`src/documents/`) reads the types through the same store. Where the Documents are kept,
//! the SQLite adapter opened at start or memory, is `lib-database`'s `DocumentsStore`.

pub mod ports;
mod router;
mod view;

pub use router::document_types_page;
pub use view::page::DocumentTypesPage;

use lib_core::DocumentTypeId;
use lib_database::SharedDocumentsStore;
use lib_documents::presenter::DocumentTypesPageEvent;

/// The Document Types page as the UI tests read it (ADR-0030): the type names in order, the
/// selected row's name and each type's Filed file count.
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsDocumentTypesSnapshot {
    pub document_type_names: Vec<String>,
    /// Each type's id, in the same order as the names.
    pub document_type_ids: Vec<DocumentTypeId>,
    pub selected_document_type: Option<String>,
    pub document_type_files: Vec<u32>,
}

/// What `shell::router::build` hands the Document Types module: the Documents store it shares with
/// the Documents page, and its keymap under `[keybindings]`.
pub struct DocumentTypesSetup {
    pub store: SharedDocumentsStore,
    pub keymap: lib_viper::keymap::Keymap<DocumentTypesPageEvent>,
}

impl DocumentTypesSetup {
    pub fn new(store: SharedDocumentsStore, config: &lib_config::KeyBindingConfig) -> Self {
        Self {
            store,
            keymap: lib_documents::keymap::document_types_page_keymap(config),
        }
    }

    /// `storage`'s Documents store under the default keys, for a test that builds `Root` directly.
    #[cfg(test)]
    pub fn sample(storage: &crate::shell::router::LedgerStores) -> Self {
        Self::new(
            storage.documents.clone(),
            &lib_config::KeyBindingConfig::default(),
        )
    }
}
