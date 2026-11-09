//! The Document Types module's Desktop half (`docs/development/viper-architecture.md`, #966): the
//! Views for the Settings › Documents page and its Dialogs, and `impl DocumentTypesRouter`. Every
//! rule, every piece of text and the keymap are `lib-documents`'; this folder turns gpui keys and
//! clicks into Presenter events and draws the view models.
//!
//! - `view/page.rs` is the page Presenter's `PageView`: the generic `ModulePage` hosts it
//!   ([`DocumentTypesPage`]), which the Settings View hosts as any other page (ADR-0056).
//! - `view/dialog.rs` and `view/dialog/` are each Dialog Presenter's `DialogView`, hosted in a
//!   `ModuleDialog`.
//! - `router.rs` is the Router and the module's entry point ([`document_types_page`]).
//!
//! The Documents destination (`src/documents/`) reads the types through the same store. Where the
//! Documents are kept, the SQLite adapter opened at start or memory, is `lib-database`'s
//! `DocumentsStore`.

mod router;
mod view;

pub use router::document_types_page;

use lib_core::DocumentTypeId;
use lib_documents::presenter::DocumentTypesPagePresenter;

use crate::shell::view::module_page::ModulePage;

/// The Document Types Settings page: the page Presenter in the generic page Entity.
pub type DocumentTypesPage = ModulePage<DocumentTypesPagePresenter>;

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

/// What the Open Ledger hands the Document Types module: the page's Presenter, which owns the
/// Document Types keymap under `[keybindings]`, `lib-database`'s Documents repository, which it
/// shares with the Documents page, and the Dialog keys every Document Types Dialog's hints read.
pub struct DocumentTypesSetup {
    pub presenter: DocumentTypesPagePresenter,
    pub documents: lib_documents::SharedRepository,
    pub dialog_keys: crate::dialog_keys::LiveDialogKeys,
}

impl DocumentTypesSetup {
    pub fn new(
        documents: lib_documents::SharedRepository,
        dialog_keys: crate::dialog_keys::LiveDialogKeys,
        config: &lib_config::KeyBindingConfig,
    ) -> Self {
        Self {
            presenter: DocumentTypesPagePresenter::new(
                documents.clone(),
                lib_documents::keymap::document_types_page_keymap(config),
            ),
            documents,
            dialog_keys,
        }
    }

    /// `storage`'s Documents store under the default keys, for a test that builds `Root` directly.
    #[cfg(test)]
    pub fn sample(storage: &crate::shell::router::LedgerStores) -> Self {
        let config = lib_config::KeyBindingConfig::default();
        Self::new(
            storage.ports.documents.clone(),
            crate::dialog_keys::LiveDialogKeys::new(lib_viper::dialog::DialogKeys::of(
                &lib_shell::dialog_keymap(&config),
            )),
            &config,
        )
    }
}

/// What the Document Types page's Router is built from for each call (its `PageView::Setup`): the
/// repository and Dialog keys its Dialogs open on.
pub struct DocumentTypesRoutes {
    pub documents: lib_documents::SharedRepository,
    pub dialog_keys: crate::dialog_keys::LiveDialogKeys,
}
