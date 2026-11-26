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
//! The ports come from `LedgerPorts`: the page and its Dialogs share `LedgerPorts::tags()`
//! (`lib_tags::SharedPorts`), and Tag remove and merge write through `LedgerPorts::tag_removal()`
//! (`lib_tag_removal::TagRemovalPorts`) (#1082). Nothing here adapts the Tags store.

mod router;
mod view;

pub use router::tags_page;

use lib_core::TagId;
use lib_core::configuration::KeyBindingConfig;
use lib_tags::SharedPorts;
use lib_tags::presenter::TagsPagePresenter;

use crate::shell::view::module_page::ModulePage;

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

/// What the Tags page's Router is built from for each call (its `PageView::Setup`): the ports
/// `LedgerPorts` built over the Tags store, Tag remove and merge's port and reads, the Dialog keys
/// every Tags Dialog's hints read, and the Clock its Transactions hand-off dates its filter by.
pub struct TagsSetup {
    pub ports: SharedPorts,
    pub removal: lib_tag_removal::TagRemovalPorts,
    pub dialog_keys: crate::dialog_keys::LiveDialogKeys,
    pub clock: lib_core::Clock,
}

impl TagsSetup {
    /// The page's Presenter, which owns the Tags keymap under `[keybindings]`, and its Setup.
    pub fn new(
        ports: SharedPorts,
        removal: lib_tag_removal::TagRemovalPorts,
        dialog_keys: crate::dialog_keys::LiveDialogKeys,
        clock: lib_core::Clock,
        config: &KeyBindingConfig,
    ) -> (TagsPagePresenter, Self) {
        let presenter = TagsPagePresenter::new(ports.clone(), config);
        let setup = Self {
            ports,
            removal,
            dialog_keys,
            clock,
        };
        (presenter, setup)
    }

    /// `ports`' Tags ports under the default keys, dated by `clock`, for a test that builds
    /// `Root` directly.
    #[cfg(test)]
    pub fn sample(
        ports: &lib_database::LedgerPorts,
        clock: &lib_core::Clock,
    ) -> (TagsPagePresenter, Self) {
        let config = KeyBindingConfig::default();
        Self::new(
            ports.tags(),
            ports.tag_removal(),
            crate::dialog_keys::LiveDialogKeys::new(lib_viper::dialog::DialogKeys::of(
                &lib_shell::dialog_keymap(&config),
            )),
            clock.clone(),
            &config,
        )
    }
}
