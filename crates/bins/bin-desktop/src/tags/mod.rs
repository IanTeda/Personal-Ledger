//! The Tags module's Desktop half (`docs/development/viper-architecture.md`, #863): the Views for
//! the Tags Settings page and its Add, Edit, Remove and Merge Dialogs, and `impl TagsRouter`. Every
//! rule, every piece of text and the keymap are `lib-tags`'; this folder turns gpui keys and clicks
//! into Presenter events and draws the view models.
//!
//! - `view/page.rs` wraps the page Presenter in an Entity, which the Settings View hosts.
//! - `view/dialog.rs` holds each Dialog's Presenter in an Entity in the Dialog host, one View per
//!   Dialog in `view/dialog/`.
//! - `router.rs` is the Router and the module's entry point ([`tags_page`]).
//! - `ports.rs` holds the store as the Presenters share it, the forwarding shim Tag remove and
//!   merge still go through, and the reassign port answered from the Ledger (#863).
//!
//! Where the Tags are kept, the SQLite adapter opened at start or memory, is `lib-database`'s
//! `TagsStore`.

pub mod ports;
mod router;
mod view;

pub use router::tags_page;
pub use view::page::TagsPage;

use lib_core::TagId;
use lib_database::SharedTagsStore;
use lib_tags::presenter::TagsPageEvent;

/// The Tags page as the UI tests read it: names A–Z, each Tag's id in the same order, and the
/// selected row's name (ADR-0030).
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsTagsSnapshot {
    pub tag_names: Vec<String>,
    pub tag_ids: Vec<TagId>,
    pub selected_tag: Option<String>,
}

/// What `shell::router::build` hands the Tags module: the store it shares with the Ledger's
/// forwarding shim and read copy, and its keymap under `[keybindings]`.
pub struct TagsSetup {
    pub store: SharedTagsStore,
    pub keymap: lib_viper::keymap::Keymap<TagsPageEvent>,
}

impl TagsSetup {
    pub fn new(store: SharedTagsStore, config: &lib_config::KeyBindingConfig) -> Self {
        Self {
            store,
            keymap: lib_tags::keymap::tags_page_keymap(config),
        }
    }

    /// `storage`'s Tags store under the default keys, for a test that builds `Root` directly.
    #[cfg(test)]
    pub fn stub(storage: &crate::ledger_store::LedgerSetup) -> Self {
        Self::new(
            storage.tags.clone(),
            &lib_config::KeyBindingConfig::default(),
        )
    }
}
