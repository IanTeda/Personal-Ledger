//! The Tags module's Desktop half (`docs/development/viper-architecture.md`, #863): the Views for
//! the Tags Settings page and its Add and Edit Dialogs, and `impl TagsRouter`. Every rule, every
//! piece of text, the keymap and the hints are `lib-tags`'; this folder draws the view models and
//! routes.
//!
//! - `view/page.rs` is the page Presenter's `PageView`: the generic `ModulePage` hosts it
//!   ([`TagsPage`]), which the Settings View hosts as any other page (ADR-0056).
//! - `view/dialog.rs` is the Add and Edit form Presenter's `DialogView`, hosted in a
//!   `ModuleDialog`.
//! - `router.rs` is the Router and the module's entry point ([`tags_page`]); it opens the Remove
//!   tag and Merge tags Dialogs through the Tag Removal module (`crate::tag_removal`, #1003).
//!
//! The ports are `lib-database`'s: the page and its Dialogs share `StoredTagsPorts` as
//! `lib_tags::SharedPorts`, and Tag remove and merge write through the `TagRemovalAdapter` built
//! beside it, both handed out by `LedgerPorts` (#1082). Nothing here adapts the Tags store.

mod router;
mod view;

pub use router::tags_page;

use lib_core::TagId;
use lib_tags::SharedPorts;
use lib_tags::presenter::TagsPagePresenter;
use lib_viper::dialog::DialogKeys;

use crate::shell::view::module_page::ModulePage;
use crate::tag_removal::RemovalStores;

/// The Tags Settings page: the page Presenter in the generic page Entity.
pub type TagsPage = ModulePage<TagsPagePresenter>;

/// The Tags page as the UI tests read it: names A–Z, each Tag's id in the same order, and the
/// selected row's name (ADR-0030).
#[doc(hidden)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsTagsSnapshot {
    pub tag_names: Vec<String>,
    pub tag_ids: Vec<TagId>,
    pub selected_tag: Option<String>,
}

/// What the Open Ledger hands the Tags module: the page's Presenter, which owns the Tags keymap
/// under `[keybindings]`, the ports `LedgerPorts` built over the Tags store, Tag remove and
/// merge's port and reads, and the Dialog keys every Tags Dialog's hints read.
pub struct TagsSetup {
    pub presenter: TagsPagePresenter,
    pub ports: SharedPorts,
    pub removal: RemovalStores,
    pub dialog_keys: DialogKeys,
}

impl TagsSetup {
    pub fn new(
        ports: SharedPorts,
        removal: RemovalStores,
        dialog_keys: DialogKeys,
        config: &lib_config::KeyBindingConfig,
    ) -> Self {
        Self {
            presenter: TagsPagePresenter::new(
                ports.clone(),
                lib_tags::keymap::tags_page_keymap(config),
            ),
            ports,
            removal,
            dialog_keys,
        }
    }

    /// `storage`'s Tags ports under the default keys, for a test that builds `Root` directly.
    #[cfg(test)]
    pub fn sample(storage: &crate::shell::router::LedgerStores) -> Self {
        let config = lib_config::KeyBindingConfig::default();
        Self::new(
            storage.ports.tags.clone(),
            RemovalStores::of(&storage.ports),
            DialogKeys::of(&lib_shell::dialog_keymap(&config)),
            &config,
        )
    }
}

/// What the Tags page's Router is built from for each call (its `PageView::Setup`): the ports and
/// Dialog keys its Dialogs open on, and the Clock its Transactions hand-off dates its filter by.
pub struct TagsRoutes {
    pub ports: SharedPorts,
    pub removal: RemovalStores,
    pub dialog_keys: DialogKeys,
    pub clock: lib_core::Clock,
}
