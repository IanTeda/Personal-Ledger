//! The Tag Removal module's Desktop half (`docs/development/viper-architecture.md`, #1003): the
//! Remove tag and Merge tags Dialogs' Views and the entry points the Tags page's Router opens them
//! through. Every rule, every piece of text and the Presenters are `lib-tag-removal`'s; this folder
//! turns gpui keys and clicks into Presenter events and draws the view models.
//!
//! - `view/` holds each Presenter's `DialogView`, the generic `ModuleDialog` hosting each in the
//!   Dialog host (ADR-0056).
//! - `router.rs` is the module's entry point ([`router::open_remove`], [`router::open_merge`]).
//!
//! Remove and merge write through [`RemovalStores`]' port, `lib-database`'s adapter over the Tags
//! and Transactions stores, which `LedgerPorts` builds beside the Tags ports (#1082), so the
//! rewritten Splits and the Tag's delete join the one pending batch and land in the one commit of
//! the confirm's input.
//! Nothing here reads the `Announcer`: the Tags page hears of a confirm through its `landed`
//! callback and brings what else reads the two domains in line.

pub mod router;
pub(crate) mod view;

use lib_database::{LedgerPorts, StoredTagRemovalReads};
use lib_tag_removal::SharedTagRemoval;

/// What the Remove tag and Merge tags Dialogs write and read through: the removal port, and the
/// reads they show when they open, both over the shared Tags and Transactions stores.
#[derive(Clone)]
pub struct RemovalStores {
    pub removal: SharedTagRemoval,
    pub reads: StoredTagRemovalReads,
}

impl RemovalStores {
    /// The port and reads `ports` built over the Tags and Transactions stores.
    pub fn of(ports: &LedgerPorts) -> Self {
        Self {
            removal: ports.tag_removal.clone(),
            reads: ports.tag_removal_reads.clone(),
        }
    }
}
