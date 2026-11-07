//! The Tag Removal module's entry points for the Tags page's Router ([`open_remove`],
//! [`open_merge`]). Each hosts its Presenter in a `ModuleDialog`, which closes through the host's
//! `HostRouter` and puts its Toast onto the bin's queue (`crate::notices`).

use gpui::{App, Context, EventEmitter};
use lib_core::TagId;
use lib_tag_removal::presenter::{MergeTagsPresenter, RemoveTagPresenter};
use lib_tags::TagsOutcome;
use lib_viper::dialog::DialogKeys;

use super::RemovalStores;
use crate::shell::view::dialog_entity::ModuleDialog;
use crate::view::event::ViewEvent;

/// Opens the Remove tag Dialog on Tag `id` over page `P`, handing what was removed to `landed`
/// once a confirm lands, so the page redraws. Its usage is read from `stores` as it opens, and its
/// hints from `keys`. Returns whether it opened: a Tag that has gone has no Dialog.
pub fn open_remove<P: EventEmitter<ViewEvent> + 'static>(
    cx: &mut Context<'_, P>,
    stores: &RemovalStores,
    id: TagId,
    keys: DialogKeys,
    landed: impl Fn(TagsOutcome, &mut App) + 'static,
) -> bool {
    let Some(presenter) = RemoveTagPresenter::new(stores.removal.clone(), &stores.reads, id, keys)
    else {
        return false;
    };
    ModuleDialog::open(cx, presenter, landed);
    true
}

/// Opens the Merge tags Dialog over page `P` with `source` and `target` chosen where given,
/// handing the target to `landed` once a confirm lands. Its options are read from `stores` as it
/// opens, and its hints from `keys`. It always opens: a Tag that has gone is only left unchosen.
pub fn open_merge<P: EventEmitter<ViewEvent> + 'static>(
    cx: &mut Context<'_, P>,
    stores: &RemovalStores,
    source: Option<TagId>,
    target: Option<TagId>,
    keys: DialogKeys,
    landed: impl Fn(TagsOutcome, &mut App) + 'static,
) -> bool {
    let presenter =
        MergeTagsPresenter::new(stores.removal.clone(), &stores.reads, source, target, keys);
    ModuleDialog::open(cx, presenter, landed);
    true
}
