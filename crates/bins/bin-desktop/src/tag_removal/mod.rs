//! The Tag Removal module's Desktop half (`docs/development/viper-architecture.md`, #1003): the
//! Remove tag and Merge tags Dialogs' Views and the entry points the Tags page's Router opens them
//! through. Every rule, every piece of text and the Presenters are `lib-tag-removal`'s; this folder
//! turns gpui keys and clicks into Presenter events and draws the view models.
//!
//! - `view/dialog.rs` holds either Presenter in an Entity in the Dialog host, one View per Dialog
//!   in `view/dialog/`.
//! - `router.rs` is the module's entry point ([`router::open_remove`], [`router::open_merge`]).
//!
//! Remove and merge write through [`RemovalStores`]' port, `lib-database`'s adapter over the Tags
//! and Transactions stores the Tags and Transactions modules share, so the rewritten Splits and the
//! Tag's delete join the one pending batch and land in the one commit of the confirm's input.
//! Nothing here reads `LedgerStore`: the Tags page hears of a confirm through its `landed`
//! callback and brings what else reads the two domains in line.

pub mod router;
pub(crate) mod view;

use std::cell::RefCell;
use std::rc::Rc;

use lib_database::{
    SharedTagsStore, SharedTransactionsStore, StoredTagRemovalReads, TagRemovalAdapter,
};
use lib_tag_removal::SharedTagRemoval;

pub use view::dialog::TagRemovalDialog;

/// What the Remove tag and Merge tags Dialogs write and read through: the removal port, and the
/// reads they show when they open, both over the shared Tags and Transactions stores.
#[derive(Clone)]
pub struct RemovalStores {
    pub removal: SharedTagRemoval,
    pub reads: StoredTagRemovalReads,
}

impl RemovalStores {
    /// The port and reads over `tags` and `transactions`.
    pub fn new(tags: SharedTagsStore, transactions: SharedTransactionsStore) -> Self {
        Self {
            removal: Rc::new(RefCell::new(TagRemovalAdapter::new(
                tags.clone(),
                transactions.clone(),
            ))),
            reads: StoredTagRemovalReads { tags, transactions },
        }
    }
}
