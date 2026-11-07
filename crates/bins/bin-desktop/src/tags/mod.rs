//! The Tags module's Desktop half (`docs/development/viper-architecture.md`, #863): the Views for
//! the Tags Settings page and its Add and Edit Dialogs, and `impl TagsRouter`. Every
//! rule, every piece of text and the keymap are `lib-tags`'; this folder turns gpui keys and clicks
//! into Presenter events and draws the view models.
//!
//! - `view/page.rs` wraps the page Presenter in an Entity, which the Settings View hosts.
//! - `view/dialog.rs` holds the Add and Edit Dialog's Presenter in an Entity in the Dialog host.
//! - `router.rs` is the Router and the module's entry point ([`tags_page`]); it opens the Remove
//!   tag and Merge tags Dialogs through the Tag Removal module (`crate::tag_removal`, #1003).
//!
//! The ports are `lib-database`'s: the page and its Dialogs share `StoredTagsPorts` as
//! `lib_tags::SharedPorts`, and Tag remove and merge write through the `TagRemovalAdapter` built
//! beside it, both handed out by `LedgerPorts` (#1082). Nothing here adapts the Tags store.

mod router;
mod view;

pub use router::tags_page;
pub use view::page::TagsPage;

use lib_core::TagId;
use lib_tags::SharedPorts;
use lib_tags::presenter::TagsPageEvent;

use crate::tag_removal::RemovalStores;

/// The Tags page as the UI tests read it: names A–Z, each Tag's id in the same order, and the
/// selected row's name (ADR-0030).
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsTagsSnapshot {
    pub tag_names: Vec<String>,
    pub tag_ids: Vec<TagId>,
    pub selected_tag: Option<String>,
}

/// What the Open Ledger hands the Tags module: the ports `LedgerPorts` built over the Tags store,
/// Tag remove and merge's port and reads, and its keymap under `[keybindings]`.
pub struct TagsSetup {
    pub ports: SharedPorts,
    pub removal: RemovalStores,
    pub keymap: lib_viper::keymap::Keymap<TagsPageEvent>,
}

impl TagsSetup {
    pub fn new(
        ports: SharedPorts,
        removal: RemovalStores,
        config: &lib_config::KeyBindingConfig,
    ) -> Self {
        Self {
            ports,
            removal,
            keymap: lib_tags::keymap::tags_page_keymap(config),
        }
    }

    /// `storage`'s Tags ports under the default keys, for a test that builds `Root` directly.
    #[cfg(test)]
    pub fn sample(storage: &crate::shell::router::LedgerStores) -> Self {
        Self::new(
            storage.ports.tags.clone(),
            RemovalStores::of(&storage.ports),
            &lib_config::KeyBindingConfig::default(),
        )
    }
}
